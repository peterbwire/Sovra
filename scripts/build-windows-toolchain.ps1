param(
    [string]$Binary = 'target\release\svr.exe',
    [string]$Output = 'target\toolchain\sovra-windows'
)
$ErrorActionPreference = 'Stop'
function Get-PackageSha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $algorithm = [Security.Cryptography.SHA256]::Create()
        try {
            return [BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace('-', '').ToLowerInvariant()
        } finally {
            $algorithm.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}
if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) { throw 'This package is for Windows.' }
$compiler = [IO.Path]::GetFullPath($Binary)
$stage = [IO.Path]::GetFullPath($Output)
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) { throw "Build svr.exe first: $compiler" }
if (Test-Path -LiteralPath $stage) { throw "Output already exists: $stage" }
$archive = "$stage.zip"
if (Test-Path -LiteralPath $archive) { throw "Archive already exists: $archive" }
New-Item -ItemType Directory -Path (Join-Path $stage 'bin') -Force | Out-Null
Copy-Item -LiteralPath $compiler -Destination (Join-Path $stage 'bin\svr.exe')
Copy-Item -LiteralPath 'toolchain\bin\svr-new.ps1' -Destination (Join-Path $stage 'bin')
Copy-Item -LiteralPath 'toolchain\install.ps1' -Destination $stage
Copy-Item -LiteralPath 'toolchain\verify-install.ps1' -Destination $stage
Copy-Item -LiteralPath 'toolchain\templates' -Destination $stage -Recurse
New-Item -ItemType Directory -Path (Join-Path $stage 'samples\task-manager') -Force | Out-Null
Copy-Item -LiteralPath 'dogfood\01-task-manager\main.svr' -Destination (Join-Path $stage 'samples\task-manager\main.svr')
Copy-Item -LiteralPath 'toolchain\samples\task-manager\README.md' -Destination (Join-Path $stage 'samples\task-manager\README.md')
$docsDestination = Join-Path $stage 'docs'
New-Item -ItemType Directory -Path $docsDestination | Out-Null
# The course belongs to the main repository; distribute operational documentation only.
Get-ChildItem -LiteralPath 'docs' | Where-Object { $_.Name -ne 'course' } |
    ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $docsDestination -Recurse }
Copy-Item -LiteralPath 'examples' -Destination $stage -Recurse
Copy-Item -LiteralPath 'toolchain\README.md' -Destination (Join-Path $stage 'README.md')
Copy-Item -LiteralPath 'LICENSE','LICENSE-MIT','LICENSE-APACHE' -Destination $stage
& (Join-Path $PSScriptRoot 'build-native-runtime-payload.ps1') -Output (Join-Path $stage 'runtime-source')
$auditJson = & .\scripts\audit-windows-imports.ps1 $compiler
$audit = $auditJson | ConvertFrom-Json
if ($audit.architecture -ne 'x86_64' -or @($audit.imports).Count -eq 0) {
    throw 'Compiler PE import audit did not identify a supported x86_64 binary.'
}
[IO.File]::WriteAllText((Join-Path $stage 'imports.json'), ($auditJson -join "`n"), [Text.UTF8Encoding]::new($false))
$stagePrefix = $stage.TrimEnd('\') + '\'
$packageFiles = @(Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
    [pscustomobject]@{
        path = $_.FullName.Substring($stagePrefix.Length).Replace('\', '/')
        sha256 = Get-PackageSha256 $_.FullName
    }
} | Sort-Object path)
$integrity = [pscustomobject]@{ schema = 1; files = $packageFiles }
[IO.File]::WriteAllText((Join-Path $stage 'integrity.json'),
    ($integrity | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
Compress-Archive -LiteralPath $stage -DestinationPath $archive
$hash = Get-PackageSha256 $archive
"$hash  $([IO.Path]::GetFileName($archive))" | Set-Content -Encoding ascii "$archive.sha256"
Write-Output "Staged $stage"
