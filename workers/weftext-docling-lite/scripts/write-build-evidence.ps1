param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [Parameter(Mandatory = $true)]
    [string]$TargetTriple,

    [Parameter(Mandatory = $true)]
    [string]$OnnxRuntimeArchivePath,

    [Parameter(Mandatory = $true)]
    [string]$ArchiveExtractRoot,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath,

    [string]$DumpbinPath = ''
)

$ErrorActionPreference = 'Stop'

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Assert-PinnedFile($File, $Pin, [string]$Label) {
    if ([uint64]$File.Length -ne [uint64]$Pin.byteLength -or
        (Get-LowerSha256 $File.FullName) -cne [string]$Pin.sha256) {
        throw "$Label bytes differ from the reviewed release profile"
    }
}

function Get-OrdinalUnique([string[]]$Values) {
    $set = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($value in $Values) {
        $null = $set.Add($value)
    }
    $result = [string[]]@($set)
    [Array]::Sort($result, [StringComparer]::Ordinal)
    return $result
}

function Find-Dumpbin([string]$RequestedPath) {
    if ($RequestedPath) {
        return (Get-Item -LiteralPath $RequestedPath).FullName
    }
    $command = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        throw 'dumpbin.exe is required to attest the Windows PE import table'
    }
    $installation = (& $vswhere -latest -products '*' -property installationPath | Select-Object -First 1)
    if (-not $installation) {
        throw 'Visual Studio Build Tools were not found for PE import attestation'
    }
    $candidate = Get-ChildItem (Join-Path $installation 'VC/Tools/MSVC') -Recurse -Filter dumpbin.exe |
        Where-Object { $_.FullName -match '[\\/]bin[\\/]Hostx64[\\/]x64[\\/]dumpbin\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if ($null -eq $candidate) {
        throw 'dumpbin.exe is required to attest the Windows PE import table'
    }
    return $candidate.FullName
}

function Get-PeImports([string]$Tool, [string]$Path) {
    $output = & $Tool /nologo /dependents $Path 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "dumpbin failed for $Path"
    }
    $imports = @(Get-OrdinalUnique @($output | ForEach-Object {
        if ($_ -match '^\s+([A-Za-z0-9._-]+\.dll)\s*$') {
            $Matches[1].ToLowerInvariant()
        }
    }))
    if ($imports.Count -eq 0) {
        throw "no PE imports were found for $Path"
    }
    return $imports
}

if ($TargetTriple -cne 'x86_64-pc-windows-msvc') {
    throw 'this evidence generator has reviewed native-runtime authority only for x86_64-pc-windows-msvc'
}

$packageRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$binary = Get-Item -LiteralPath $BinaryPath
$manifest = Get-Item -LiteralPath (Join-Path $packageRoot 'Cargo.toml')
$lock = Get-Item -LiteralPath (Join-Path $packageRoot 'Cargo.lock')
$profileFile = Get-Item -LiteralPath (Join-Path $packageRoot 'release-profile.json')
$profile = Get-Content -Raw -LiteralPath $profileFile.FullName | ConvertFrom-Json -Depth 32
$native = $profile.reviewedNativeRuntime
if ($profile.schemaVersion -cne 'weftext.docling-lite-release-profile.v2' -or
    $native.target -cne $TargetTriple -or
    $native.version -cne '1.24.2' -or
    $native.rustBindingVersion -cne '2.0.0-rc.12' -or
    $native.linkage -cne 'dynamic') {
    throw 'release-profile.json does not describe the reviewed Windows dynamic native runtime'
}

$manifestText = Get-Content -Raw -LiteralPath $manifest.FullName
$lockText = Get-Content -Raw -LiteralPath $lock.FullName
$directRegistryDependencies = [ordered]@{
    docling = '0.52.2'
    'docling-core' = '0.52.2'
    'docling-pdf' = '0.52.2'
    ort = '2.0.0-rc.12'
    'ort-sys' = '2.0.0-rc.12'
    serde = '1.0.229'
    serde_json = '1.0.151'
    sha2 = '0.10.9'
}
foreach ($entry in $directRegistryDependencies.GetEnumerator()) {
    $escapedName = [Regex]::Escape($entry.Key)
    $escapedVersion = [Regex]::Escape($entry.Value)
    $manifestPattern = '(?m)^\s*' + $escapedName + '\s*=\s*(?:\{\s*version\s*=\s*)?"=' + $escapedVersion + '"'
    if (-not [Regex]::IsMatch($manifestText, $manifestPattern)) {
        throw "Cargo.toml does not exactly pin direct dependency $($entry.Key) to $($entry.Value)"
    }
    $lockPattern = '(?ms)^\[\[package\]\]\r?\nname = "' + $escapedName + '"\r?\nversion = "' + $escapedVersion + '"'
    if (-not [Regex]::IsMatch($lockText, $lockPattern)) {
        throw "Cargo.lock does not resolve $($entry.Key) to $($entry.Value)"
    }
}

$archive = Get-Item -LiteralPath $OnnxRuntimeArchivePath
if ($archive.Name -cne [string]$native.sourceArchive.artifact) {
    throw 'ONNX Runtime archive filename differs from the reviewed pin'
}
Assert-PinnedFile $archive $native.sourceArchive 'ONNX Runtime source archive'
$extractRoot = (Resolve-Path -LiteralPath $ArchiveExtractRoot).Path
$archiveFiles = @{}
foreach ($pin in $native.archiveFiles) {
    $file = Get-Item -LiteralPath (Join-Path $extractRoot ([string]$pin.path))
    Assert-PinnedFile $file $pin "ONNX Runtime archive member $($pin.path)"
    $archiveFiles[[string]$pin.role] = $file
}
if ($archiveFiles.Count -ne 2 -or
    -not $archiveFiles.ContainsKey('runtime_library') -or
    -not $archiveFiles.ContainsKey('import_library')) {
    throw 'the reviewed native runtime must bind one runtime DLL and one import library'
}

$expectedOrtLibPath = $archiveFiles['import_library'].Directory.FullName
$actualOrtLibPath = (Resolve-Path -LiteralPath $env:ORT_LIB_PATH).Path
if (-not [string]::Equals($actualOrtLibPath, $expectedOrtLibPath, [StringComparison]::OrdinalIgnoreCase) -or
    $env:ORT_PREFER_DYNAMIC_LINK -cne '1' -or
    $env:ORT_SKIP_DOWNLOAD -cne '1' -or
    $env:CARGO_NET_OFFLINE -cne 'true') {
    throw 'the build environment did not enforce the reviewed offline dynamic ONNX Runtime override'
}

$assetRoot = $binary.Directory.FullName
$installedNative = Get-Item -LiteralPath (Join-Path $assetRoot ([string]$native.installPath))
Assert-PinnedFile $installedNative ($native.archiveFiles | Where-Object role -CEQ 'runtime_library') 'installed ONNX Runtime DLL'

$runtimeAssetPaths = [ordered]@{
    pdfium = Join-Path $assetRoot '.pdfium/lib/pdfium.dll'
    'layout-int8' = Join-Path $assetRoot 'models/layout_heron_int8.onnx'
    'pp-ocr' = Join-Path $assetRoot 'models/ocr_rec_en.onnx'
    'ocr-dictionary' = Join-Path $assetRoot 'models/en_dict.txt'
}
$reviewedAssets = @{}
foreach ($pin in $profile.reviewedExtractedArtifacts) {
    if ($pin.target -ceq 'all' -or $pin.target -ceq $TargetTriple) {
        if ($reviewedAssets.ContainsKey([string]$pin.component)) {
            throw "release-profile.json repeats reviewed asset $($pin.component) for $TargetTriple"
        }
        $reviewedAssets[[string]$pin.component] = $pin
    }
}
$runtimeAssets = @()
foreach ($entry in $runtimeAssetPaths.GetEnumerator()) {
    $asset = Get-Item -LiteralPath $entry.Value
    if (-not $reviewedAssets.ContainsKey([string]$entry.Key)) {
        throw "release-profile.json does not pin runtime asset $($entry.Key) for $TargetTriple"
    }
    Assert-PinnedFile $asset $reviewedAssets[[string]$entry.Key] "runtime asset $($entry.Key)"
    $runtimeAssets += [ordered]@{
        component = $entry.Key
        installPath = [IO.Path]::GetRelativePath($assetRoot, $asset.FullName).Replace('\', '/')
        byteLength = [uint64]$asset.Length
        sha256 = Get-LowerSha256 $asset.FullName
    }
}

$dumpbin = Find-Dumpbin $DumpbinPath
$workerImports = @(Get-PeImports $dumpbin $binary.FullName)
$nativeImports = @(Get-PeImports $dumpbin $installedNative.FullName)
$forbiddenImports = @(Get-OrdinalUnique @($native.forbiddenImports | ForEach-Object { ([string]$_).ToLowerInvariant() }))
if ($workerImports -notcontains 'onnxruntime.dll') {
    throw 'the worker is not dynamically linked to the reviewed ONNX Runtime DLL'
}
foreach ($forbidden in $forbiddenImports) {
    if ($workerImports -contains $forbidden -or $nativeImports -contains $forbidden) {
        throw "the reviewed CPU package imports forbidden native dependency $forbidden"
    }
}

$allExternalImports = @(Get-OrdinalUnique @($workerImports + $nativeImports))
$packagedImports = @($allExternalImports | Where-Object { $_ -ceq 'onnxruntime.dll' })
$unboundMicrosoftRuntimeImports = @(Get-OrdinalUnique @($allExternalImports | Where-Object {
    $_ -match '^(?:msvcp|vcruntime)[0-9_]*\.dll$' -or $_ -match '^api-ms-win-crt-'
}))
$windowsSystemImports = @(Get-OrdinalUnique @($allExternalImports | Where-Object {
    $_ -cne 'onnxruntime.dll' -and
    $_ -notmatch '^(?:msvcp|vcruntime)[0-9_]*\.dll$' -and
    $_ -notmatch '^api-ms-win-crt-'
}))
if ($unboundMicrosoftRuntimeImports.Count -eq 0) {
    throw 'expected MSVC/UCRT imports were not present; update the closed dependency classification'
}

$physicalPackageBytes = [uint64]$binary.Length + [uint64]$installedNative.Length
foreach ($asset in $runtimeAssets) {
    $physicalPackageBytes += [uint64]$asset.byteLength
}
$packageGap = 'Microsoft Visual C++/Universal CRT imports are enumerated but not packaged or digest-bound'
$noticeGap = 'redistributable full license and notice files are not staged and digest-bound in this target package'
$sandboxGap = 'versioned target OS sandbox evidence for network, memory, filesystem, process-tree, and cancellation controls'

$evidence = [ordered]@{
    schemaVersion = 'weftext.docling-lite-build-evidence.v2'
    target = $TargetTriple
    toolchain = [ordered]@{
        rustc = (& rustc '+1.98.0' '--version').Trim()
        cargo = (& cargo '+1.98.0' '--version').Trim()
    }
    doclingSource = $profile.source
    fixedLiteProfile = [ordered]@{
        formats = @('pdf')
        ocrLanguage = 'en'
        layoutPrecision = 'int8'
        tableFormer = $false
        networkFeatures = $false
    }
    buildEnvironment = [ordered]@{
        cargoCommand = 'cargo +1.98.0 build --release --locked --offline --target x86_64-pc-windows-msvc'
        ORT_LIB_PATH = [string]$native.buildEnvironment.ORT_LIB_PATH
        ORT_PREFER_DYNAMIC_LINK = [string]$native.buildEnvironment.ORT_PREFER_DYNAMIC_LINK
        ORT_SKIP_DOWNLOAD = [string]$native.buildEnvironment.ORT_SKIP_DOWNLOAD
        CARGO_NET_OFFLINE = [string]$native.buildEnvironment.CARGO_NET_OFFLINE
    }
    worker = [ordered]@{
        component = 'docling-rs'
        installPath = $binary.Name
        fileName = $binary.Name
        byteLength = [uint64]$binary.Length
        sha256 = Get-LowerSha256 $binary.FullName
    }
    workerSourceFiles = @(@('src/lib.rs', 'src/main.rs') | ForEach-Object {
        $sourceFile = Get-Item -LiteralPath (Join-Path $packageRoot $_)
        [ordered]@{
            path = $_
            byteLength = [uint64]$sourceFile.Length
            sha256 = Get-LowerSha256 $sourceFile.FullName
        }
    })
    nativeRuntime = [ordered]@{
        component = [string]$native.component
        implementation = [string]$native.implementation
        version = [string]$native.version
        linkage = [string]$native.linkage
        installPath = [string]$native.installPath
        byteLength = [uint64]$installedNative.Length
        sha256 = Get-LowerSha256 $installedNative.FullName
        rustBinding = [ordered]@{
            crate = [string]$native.rustBindingCrate
            version = [string]$native.rustBindingVersion
            sysCrate = 'ort-sys'
            sysVersion = [string]$profile.exactPrereleasePins.'ort-sys'
        }
        sourceArchive = [ordered]@{
            artifact = [string]$native.sourceArchive.artifact
            sourceUrl = [string]$native.sourceArchive.sourceUrl
            byteLength = [uint64]$archive.Length
            sha256 = Get-LowerSha256 $archive.FullName
        }
        importLibrary = [ordered]@{
            archivePath = [IO.Path]::GetRelativePath($extractRoot, $archiveFiles['import_library'].FullName).Replace('\', '/')
            byteLength = [uint64]$archiveFiles['import_library'].Length
            sha256 = Get-LowerSha256 $archiveFiles['import_library'].FullName
        }
        workerImports = $workerImports
        runtimeImports = $nativeImports
        forbiddenImportsAbsent = $forbiddenImports
    }
    externalRuntimeImports = [ordered]@{
        packaged = $packagedImports
        windowsSystem = $windowsSystemImports
        unboundMicrosoftRuntime = $unboundMicrosoftRuntimeImports
    }
    directRegistryDependencies = @($directRegistryDependencies.GetEnumerator() | ForEach-Object {
        [ordered]@{ name = $_.Key; version = $_.Value }
    })
    cargoManifestSha256 = Get-LowerSha256 $manifest.FullName
    cargoLockSha256 = Get-LowerSha256 $lock.FullName
    releaseProfileSha256 = Get-LowerSha256 $profileFile.FullName
    runtimeAssets = $runtimeAssets
    physicalPackageBytes = $physicalPackageBytes
    packageComplete = $false
    missingForPackage = @($packageGap, $noticeGap)
    osSandboxEvidence = $null
    sandboxComplete = $false
    completeForExecution = $false
    missingForExecution = @($packageGap, $noticeGap, $sandboxGap)
}

$parent = Split-Path -Parent $OutputPath
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}
$evidenceJson = (($evidence | ConvertTo-Json -Depth 16) -replace "`r`n", "`n") + "`n"
[IO.File]::WriteAllText($OutputPath, $evidenceJson, [Text.UTF8Encoding]::new($false))
