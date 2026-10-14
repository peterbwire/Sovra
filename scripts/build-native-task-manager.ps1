param(
    [string]$Binary = 'target\release\svr.exe',
    [string]$Output = 'target\native-task-manager\sovra-task-manager'
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) {
    throw 'The native Task Manager artifact is currently Windows-only.'
}
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$source = Join-Path $repository 'dogfood\01-task-manager\main.svr'
$compiler = [IO.Path]::GetFullPath($Binary)
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) { throw "Build svr.exe first: $compiler" }
$targetRoot = [IO.Path]::GetFullPath((Join-Path $repository 'target'))
$outputPath = [IO.Path]::GetFullPath($Output)
$archive = "$outputPath.zip"
if (Test-Path -LiteralPath $outputPath) { throw "Output already exists: $outputPath" }
if (Test-Path -LiteralPath $archive) { throw "Archive already exists: $archive" }
$build = Join-Path $targetRoot ('.sovra-native-build-' + [guid]::NewGuid().ToString('N'))
$build = [IO.Path]::GetFullPath($build)
if (-not $build.StartsWith($targetRoot.TrimEnd('\') + '\',
        [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unsafe native build directory.'
}
New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
try {
    New-Item -ItemType Directory -Path $build | Out-Null
    $binary = Join-Path $build 'task-manager.exe'
    & $compiler build --emit exe --output $binary $source | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Native Task Manager build failed.' }
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Native builder produced no executable: $binary"
    }
    $auditJson = & (Join-Path $PSScriptRoot 'audit-windows-imports.ps1') $binary
    $audit = $auditJson | ConvertFrom-Json
    if ($audit.architecture -ne 'x86_64' -or @($audit.imports).Count -eq 0) {
        throw 'Native executable import audit failed.'
    }
    New-Item -ItemType Directory -Path (Split-Path -Parent $outputPath) -Force | Out-Null
    New-Item -ItemType Directory -Path $outputPath | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $outputPath 'task-manager.exe')
    Copy-Item -LiteralPath (Join-Path $repository 'toolchain\native-task-manager\verify.ps1') `
        -Destination $outputPath
    Copy-Item -LiteralPath (Join-Path $repository 'toolchain\native-task-manager\README.md') `
        -Destination $outputPath
    [IO.File]::WriteAllText((Join-Path $outputPath 'imports.json'),
        ($auditJson -join "`n"), [Text.UTF8Encoding]::new($false))
    $compilerVersion = (& $compiler --version) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Cannot obtain compiler version.' }
    $provenance = [pscustomobject]@{
        schema = 1
        method = 'svr build --emit exe --output'
        compiler_version = $compilerVersion.Trim()
        compiler_sha256 = (Get-FileHash -LiteralPath $compiler -Algorithm SHA256).Hash.ToLowerInvariant()
        source_sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
        architecture = $audit.architecture
    }
    [IO.File]::WriteAllText((Join-Path $outputPath 'build.json'),
        ($provenance | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
    $hash = (Get-FileHash -LiteralPath (Join-Path $outputPath 'task-manager.exe') `
        -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $outputPath 'task-manager.exe.sha256'),
        "$hash  task-manager.exe`n", [Text.Encoding]::ASCII)
    Compress-Archive -LiteralPath $outputPath -DestinationPath $archive
    $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$archive.sha256",
        "$archiveHash  $([IO.Path]::GetFileName($archive))`n", [Text.Encoding]::ASCII)
    Write-Output "Built public-CLI native Task Manager qualification artifact: $archive"
} finally {
    if (Test-Path -LiteralPath $build) { Remove-Item -LiteralPath $build -Recurse -Force }
}
