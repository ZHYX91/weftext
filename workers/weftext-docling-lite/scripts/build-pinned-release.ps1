param(
    [Parameter(Mandatory = $true)]
    [string]$OnnxRuntimeArchivePath,

    [Parameter(Mandatory = $true)]
    [string]$ArchiveExtractRoot,

    [Parameter(Mandatory = $true)]
    [string]$AssetRoot,

    [Parameter(Mandatory = $true)]
    [string]$CargoTargetDirectory,

    [Parameter(Mandatory = $true)]
    [string]$EvidenceOutputPath
)

$ErrorActionPreference = 'Stop'
$target = 'x86_64-pc-windows-msvc'
$packageRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$archive = Get-Item -LiteralPath $OnnxRuntimeArchivePath
$extractRoot = [IO.Path]::GetFullPath($ArchiveExtractRoot)
$targetRoot = [IO.Path]::GetFullPath($CargoTargetDirectory)
$assets = (Resolve-Path -LiteralPath $AssetRoot).Path
$profile = Get-Content -Raw -LiteralPath (Join-Path $packageRoot 'release-profile.json') | ConvertFrom-Json -Depth 32

if ($archive.Name -cne [string]$profile.reviewedNativeRuntime.sourceArchive.artifact -or
    [uint64]$archive.Length -ne [uint64]$profile.reviewedNativeRuntime.sourceArchive.byteLength -or
    (Get-FileHash -Algorithm SHA256 -LiteralPath $archive.FullName).Hash.ToLowerInvariant() -cne [string]$profile.reviewedNativeRuntime.sourceArchive.sha256) {
    throw 'the supplied ONNX Runtime archive does not match release-profile.json'
}

New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
Expand-Archive -LiteralPath $archive.FullName -DestinationPath $extractRoot -Force
$ortLibraryDirectory = Join-Path $extractRoot 'onnxruntime-win-x64-1.24.2/lib'
$env:ORT_LIB_PATH = (Resolve-Path -LiteralPath $ortLibraryDirectory).Path
$env:ORT_PREFER_DYNAMIC_LINK = '1'
$env:ORT_SKIP_DOWNLOAD = '1'
$env:CARGO_NET_OFFLINE = 'true'
$env:CARGO_TARGET_DIR = $targetRoot
$env:PATH = "$($env:ORT_LIB_PATH);$($env:PATH)"

Push-Location $packageRoot
try {
    & cargo '+1.98.0' 'fmt' '--all' '--' '--check'
    if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed' }
    & cargo '+1.98.0' 'check' '--all-targets' '--locked' '--offline' '--target' $target
    if ($LASTEXITCODE -ne 0) { throw 'cargo check failed' }
    & cargo '+1.98.0' 'test' '--all-targets' '--locked' '--offline' '--target' $target
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
    & cargo '+1.98.0' 'clippy' '--all-targets' '--locked' '--offline' '--target' $target '--' '-D' 'warnings'
    if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed' }
    & cargo '+1.98.0' 'build' '--release' '--locked' '--offline' '--target' $target
    if ($LASTEXITCODE -ne 0) { throw 'cargo release build failed' }
}
finally {
    Pop-Location
}

$releaseRoot = Join-Path $targetRoot "$target/release"
New-Item -ItemType Directory -Force -Path (Join-Path $releaseRoot 'models') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $releaseRoot '.pdfium/lib') | Out-Null
Copy-Item -LiteralPath (Join-Path $env:ORT_LIB_PATH 'onnxruntime.dll') -Destination (Join-Path $releaseRoot 'onnxruntime.dll') -Force
Copy-Item -LiteralPath (Join-Path $assets 'models/layout_heron_int8.onnx') -Destination (Join-Path $releaseRoot 'models/layout_heron_int8.onnx') -Force
Copy-Item -LiteralPath (Join-Path $assets 'models/ocr_rec_en.onnx') -Destination (Join-Path $releaseRoot 'models/ocr_rec_en.onnx') -Force
Copy-Item -LiteralPath (Join-Path $assets 'models/en_dict.txt') -Destination (Join-Path $releaseRoot 'models/en_dict.txt') -Force
Copy-Item -LiteralPath (Join-Path $assets '.pdfium/lib/pdfium.dll') -Destination (Join-Path $releaseRoot '.pdfium/lib/pdfium.dll') -Force

& (Join-Path $PSScriptRoot 'write-build-evidence.ps1') `
    -BinaryPath (Join-Path $releaseRoot 'weftext-docling-lite.exe') `
    -TargetTriple $target `
    -OnnxRuntimeArchivePath $archive.FullName `
    -ArchiveExtractRoot $extractRoot `
    -OutputPath $EvidenceOutputPath
if ($LASTEXITCODE -ne 0) {
    throw 'build evidence generation failed'
}
