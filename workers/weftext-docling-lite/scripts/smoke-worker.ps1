param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [Parameter(Mandatory = $true)]
    [string]$SourcePdf,

    [ValidateRange(1, 1000000)]
    [int]$PageLimit = 1000,

    [ValidateRange(4096, 1073741824)]
    [uint64]$OutputByteLimit = 536870912,

    [ValidateSet('automatic', 'always')]
    [string]$LocalOcrPolicy = 'automatic',

    [ValidateRange(0, 300000)]
    [int]$CancelAfterMilliseconds = 0,

    [string]$ExpectedFailureCode = ''
)

$ErrorActionPreference = 'Stop'

$binary = Get-Item -LiteralPath $BinaryPath
$source = Get-Item -LiteralPath $SourcePdf
$assetRoot = $binary.Directory.FullName
$session = Join-Path $assetRoot (Join-Path 'smoke-sessions' ([guid]::NewGuid().ToString('N')))
$inputDirectory = Join-Path $session 'input'
New-Item -ItemType Directory -Force -Path $inputDirectory | Out-Null
Copy-Item -LiteralPath $source.FullName -Destination (Join-Path $inputDirectory 'source.pdf')

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

$sourceDigest = Get-LowerSha256 $source.FullName
$workerDigest = Get-LowerSha256 $binary.FullName
$onnxRuntimePath = Join-Path $assetRoot 'onnxruntime.dll'
$pdfiumPath = Join-Path $assetRoot '.pdfium/lib/pdfium.dll'
$layoutPath = Join-Path $assetRoot 'models/layout_heron_int8.onnx'
$ocrPath = Join-Path $assetRoot 'models/ocr_rec_en.onnx'
$dictionaryPath = Join-Path $assetRoot 'models/en_dict.txt'

$pins = @(
    [ordered]@{ component = 'docling-rs'; version = '0.52.2-weftext-lite.1'; sha256 = $workerDigest; noticeId = 'docling-rs-0.52.2' },
    [ordered]@{ component = 'pdfium'; version = 'chromium-8009'; sha256 = (Get-LowerSha256 $pdfiumPath); noticeId = 'pdfium-chromium-8009' },
    [ordered]@{ component = 'onnx-runtime'; version = '1.24.2-cpu-x64'; sha256 = (Get-LowerSha256 $onnxRuntimePath); noticeId = 'onnx-runtime-1.24.2' },
    [ordered]@{ component = 'layout-int8'; version = 'models-v1-layout-heron-int8'; sha256 = (Get-LowerSha256 $layoutPath); noticeId = 'docling-layout-heron' },
    [ordered]@{ component = 'pp-ocr'; version = 'PP-OCRv3-en'; sha256 = (Get-LowerSha256 $ocrPath); noticeId = 'rapidocr-paddleocr-ppocrv3-en' },
    [ordered]@{ component = 'ocr-dictionary'; version = 'PaddleOCR-main-reviewed-bytes'; sha256 = (Get-LowerSha256 $dictionaryPath); noticeId = 'paddleocr-english-dictionary' }
)

$limits = [ordered]@{
    maxSourceBytes = [uint64]$source.Length
    maxProbeBytes = [uint64][Math]::Min(65536, $source.Length)
    maxPages = $PageLimit
    maxContainerEntries = 10000
    maxIrNodes = 100000
    maxIrDepth = 64
    maxTextBytes = 33554432
    maxResourceCount = 5000
    maxResourceBytes = [uint64][Math]::Min(67108864, $OutputByteLimit)
    maxTotalOutputBytes = $OutputByteLimit
    maxDiagnostics = 10000
    maxAgentSelectedNodes = 1000
    maxAgentOperations = 1000
    maxAgentOutputBytes = [uint64][Math]::Min(4194304, $OutputByteLimit)
    workerMemoryBytes = 2147483648
    workerTimeoutMs = 300000
    cancellationGraceMs = 1000
}

$command = [ordered]@{
    protocolVersion = 'weftext.docling-lite-worker-json.v1'
    requestId = 'request-release-smoke'
    sourceDigest = $sourceDigest
    planId = 'plan-release-smoke'
    inputLocator = 'input/source.pdf'
    outputLocator = 'output/docling-document.json'
    doclingReleaseTag = 'v0.52.2'
    doclingReleaseCommit = 'ca9fe7a543b55a540dfa18b88f4f44591b5a928e'
    documentSchemaName = 'DoclingDocument'
    documentSchemaVersion = '1.10.0'
    target = 'x86_64-pc-windows-msvc'
    localOcrPolicy = $LocalOcrPolicy
    ocrLanguage = 'en'
    layoutPrecision = 'int8'
    noTableFormer = $true
    network = 'denied'
    pageLimit = $PageLimit
    memoryLimitBytes = 2147483648
    outputByteLimit = $OutputByteLimit
    modelPins = $pins
}

$request = [ordered]@{
    contractVersion = 'weftext.import-worker-request.v1'
    requestId = 'request-release-smoke'
    workerId = 'weftext.docling-lite-worker'
    workerProtocolVersion = 'weftext.docling-lite-worker-json.v1'
    source = [ordered]@{
        contractVersion = 'weftext.import.source-artifact.v1'
        sourceId = "source-$($sourceDigest.Substring(0, 24))"
        displayName = $source.Name
        origin = 'test_fixture'
        byteLength = [uint64]$source.Length
        sha256 = $sourceDigest
        extensionHint = 'pdf'
        detectedFormat = 'pdf'
        mismatchEvidence = @()
    }
    sourceLocator = 'input/source.pdf'
    plan = [ordered]@{
        contractVersion = 'weftext.import.plan.v1'
        planId = 'plan-release-smoke'
        proposedRootId = '550e8400-e29b-41d4-a716-446655440000'
        sourceDigest = $sourceDigest
        probeDigest = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
        route = [ordered]@{
            adapter = [ordered]@{
                adapterId = 'weftext.pdf-docling-lite-adapter'
                adapterVersion = '0.52.2-lock-release-smoke'
                supportedFormat = 'pdf'
            }
            workerId = 'weftext.docling-lite-worker'
            workerProtocolVersion = 'weftext.docling-lite-worker-json.v1'
        }
        destination = 'Imported'
        splitPolicy = 'single_node'
        resourcePolicy = 'extract_referenced'
        localOcrPolicy = $LocalOcrPolicy
        agentEnhancement = [ordered]@{ mode = 'disabled' }
        limits = $limits
        egress = [ordered]@{ mode = 'none' }
    }
    network = 'denied'
    memoryLimitBytes = 2147483648
    pageLimit = $PageLimit
    entryLimit = 10000
    outputByteLimit = $OutputByteLimit
    formatOptions = $command
}

$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = $binary.FullName
$start.WorkingDirectory = $session
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$process = [Diagnostics.Process]::new()
$process.StartInfo = $start
$stopwatch = [Diagnostics.Stopwatch]::StartNew()
$null = $process.Start()
$stdoutTask = $process.StandardOutput.ReadToEndAsync()
$stderrTask = $process.StandardError.ReadToEndAsync()
$process.StandardInput.Write(($request | ConvertTo-Json -Depth 20 -Compress))
$process.StandardInput.Close()
$cancelled = $false
if ($CancelAfterMilliseconds -gt 0) {
    Start-Sleep -Milliseconds $CancelAfterMilliseconds
    if (-not $process.HasExited) {
        $process.Kill($true)
        $cancelled = $true
    }
}
$process.WaitForExit()
$stopwatch.Stop()
$stdout = $stdoutTask.GetAwaiter().GetResult()
$stderr = $stderrTask.GetAwaiter().GetResult()
if ($CancelAfterMilliseconds -gt 0) {
    $residualFiles = @(Get-ChildItem -LiteralPath $session -File -Recurse |
        Where-Object { $_.FullName -ne (Join-Path $inputDirectory 'source.pdf') })
    [pscustomobject]@{
        Source = $source.Name
        ElapsedMilliseconds = $stopwatch.ElapsedMilliseconds
        CancellationRequested = $true
        ProcessTreeKilled = $cancelled
        ProcessExited = $process.HasExited
        ExitCode = $process.ExitCode
        ResidualFileCount = $residualFiles.Count
        Session = $session
    }
    return
}
if ($process.ExitCode -ne 0) {
    throw "worker exited $($process.ExitCode): $stderr"
}
$response = $stdout | ConvertFrom-Json -Depth 100
$isTypedResponse = $response.psobject.Properties.Name -contains 'contractVersion'
if ($isTypedResponse) {
    if ($response.contractVersion -cne 'weftext.import-worker-response.v1' -or
        $response.requestId -cne $request.requestId -or
        $response.workerId -cne $request.workerId -or
        $response.workerProtocolVersion -cne $request.workerProtocolVersion -or
        $response.sourceDigest -cne $sourceDigest -or
        $response.payload.status -cne 'failed' -or
        $null -ne $response.payload.doclingDocumentJson) {
        throw "worker emitted an invalid typed failure response"
    }
    $expectedPins = @{}
    foreach ($pin in $pins) {
        $expectedPins[$pin.component] = $pin
    }
    if ($response.components.Count -ne $expectedPins.Count) {
        throw "worker failure response did not echo all six component pins"
    }
    foreach ($component in $response.components) {
        $expected = $expectedPins[$component.componentId]
        if ($null -eq $expected -or
            $component.version -cne $expected.version -or
            $component.artifactDigest -cne $expected.sha256) {
            throw "worker failure response component evidence differs from the request pin"
        }
    }
    $actualFailureCode = $response.diagnostics[0].code
    if (-not $ExpectedFailureCode) {
        throw "worker returned failure: $actualFailureCode"
    }
    if ($actualFailureCode -ne $ExpectedFailureCode) {
        throw "worker returned failure '$actualFailureCode', expected '$ExpectedFailureCode'"
    }
    [pscustomobject]@{
        Source = $source.Name
        ElapsedMilliseconds = $stopwatch.ElapsedMilliseconds
        ResponseBytes = [Text.Encoding]::UTF8.GetByteCount($stdout)
        Status = 'failed'
        DiagnosticCode = $actualFailureCode
        ComponentCount = $response.components.Count
        ResponseShape = 'typed_failure'
        Session = $session
    }
    return
}
if ($ExpectedFailureCode) {
    throw "worker succeeded, expected failure '$ExpectedFailureCode'"
}
if ($response.schema_name -cne 'DoclingDocument' -or $response.version -cne '1.10.0') {
    throw "worker success stdout is not the pinned raw DoclingDocument"
}
if ($response.psobject.Properties.Name -contains 'payload') {
    throw "worker success stdout must not use a Weftext response envelope"
}

[pscustomobject]@{
    Source = $source.Name
    ElapsedMilliseconds = $stopwatch.ElapsedMilliseconds
    ResponseBytes = [Text.Encoding]::UTF8.GetByteCount($stdout)
    SchemaName = $response.schema_name
    SchemaVersion = $response.version
    PageCount = @($response.pages.psobject.Properties).Count
    TextItemCount = @($response.texts).Count
    TableCount = @($response.tables).Count
    PictureCount = @($response.pictures).Count
    RequestPinCount = $pins.Count
    ResponseShape = 'raw_docling_document'
    Session = $session
}
