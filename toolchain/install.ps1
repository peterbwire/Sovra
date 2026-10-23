param(
    [string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Sovra'),
    [switch]$NoPersistPath
)
$ErrorActionPreference = 'Stop'
# Keep the installer self-contained; optional PowerShell hashing functions may
# be unavailable even though the .NET runtime supports SHA-256.
function Get-InstallSha256([string]$Path) {
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
$source = [IO.Path]::GetFullPath($PSScriptRoot)
$target = [IO.Path]::GetFullPath($Prefix)
$sourcePrefix = $source.TrimEnd('\') + '\'
$targetPrefix = $target.TrimEnd('\') + '\'
if ($targetPrefix.StartsWith($sourcePrefix, [StringComparison]::OrdinalIgnoreCase) -or
    $sourcePrefix.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Install prefix must not overlap the package directory.'
}
foreach ($relative in @('bin\svr.exe','bin\svr-new.ps1','templates\console\main.svr',
        'templates\console\sovra.toml','docs\spec.md','examples\local-packages\app\main.svr',
        'samples\task-manager\main.svr','README.md','verify-install.ps1',
        'imports.json','integrity.json','LICENSE','LICENSE-MIT','LICENSE-APACHE',
        'runtime-source\payload.json','runtime-source\payload.sha256','runtime-source\Cargo.toml','runtime-source\Cargo.lock',
        'runtime-source\src\lib.rs','runtime-source\examples\native_builder.rs',
        'runtime-source\toolchain\templates\console\main.svr',
        'runtime-source\toolchain\templates\console\sovra.toml')) {
    if (-not (Test-Path -LiteralPath (Join-Path $source $relative) -PathType Leaf)) {
        throw "Package is missing $relative."
    }
}
$integrity = Get-Content -LiteralPath (Join-Path $source 'integrity.json') -Raw | ConvertFrom-Json
if ($integrity.schema -ne 1) { throw 'Unsupported package integrity manifest.' }
$sourcePrefix = $source.TrimEnd('\') + '\'
$actualFiles = @(Get-ChildItem -LiteralPath $source -File -Recurse |
    Where-Object { $_.FullName -ne (Join-Path $source 'integrity.json') } |
    ForEach-Object {
        [pscustomobject]@{
            path = $_.FullName.Substring($sourcePrefix.Length).Replace('\', '/')
            fullName = $_.FullName
        }
    } | Sort-Object path)
$expectedFiles = @($integrity.files | Sort-Object path)
if ($actualFiles.Count -ne $expectedFiles.Count) {
    throw 'Package integrity file list mismatch.'
}
for ($index = 0; $index -lt $actualFiles.Count; $index++) {
    $actual = $actualFiles[$index]
    $expected = $expectedFiles[$index]
    if ($actual.path -cne $expected.path -or $expected.sha256 -cnotmatch '^[0-9a-f]{64}$') {
        throw 'Package integrity file list mismatch.'
    }
    $hash = Get-InstallSha256 $actual.fullName
    if ($hash -cne $expected.sha256) { throw "Package integrity mismatch: $($actual.path)" }
}
if (Test-Path -LiteralPath $target) { throw "Install prefix already exists: $target" }
$parent = [IO.Path]::GetFullPath((Split-Path -Parent $target)).TrimEnd('\')
New-Item -ItemType Directory -Path $parent -Force | Out-Null
$stage = [IO.Path]::GetFullPath((Join-Path $parent ('.sovra-install-' + [guid]::NewGuid().ToString('N'))))
if (-not $stage.StartsWith($parent + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unsafe installation staging path.'
}
try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    foreach ($directory in @('bin','templates','samples','docs','examples','runtime-source')) {
        Copy-Item -LiteralPath (Join-Path $source $directory) -Destination $stage -Recurse
    }
    foreach ($file in @('README.md','install.ps1','verify-install.ps1','imports.json','integrity.json',
            'LICENSE','LICENSE-MIT','LICENSE-APACHE')) {
        Copy-Item -LiteralPath (Join-Path $source $file) -Destination $stage
    }
    if (Test-Path -LiteralPath $target) { throw "Install prefix already exists: $target" }
    Move-Item -LiteralPath $stage -Destination $target
} finally {
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
}
$bin = Join-Path $target 'bin'
if (-not $NoPersistPath) {
    $current = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($current -split ';' | Where-Object { $_ })
    if (-not ($entries | Where-Object { $_.TrimEnd('\') -ieq $bin.TrimEnd('\') })) {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $bin) -join ';'), 'User')
    }
}
$env:Path = "$bin;$env:Path"
Write-Output "Installed Sovra in $target"
Write-Output 'Open a new PowerShell window to use svr and svr-new from PATH.'
