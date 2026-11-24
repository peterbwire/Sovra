param(
    [string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Sovra'),
    [switch]$NoPersistPath,
    [switch]$Upgrade
)
$ErrorActionPreference = 'Stop'
$core = Join-Path $PSScriptRoot 'install-core.ps1'
if (-not (Test-Path -LiteralPath $core -PathType Leaf)) { throw 'Package is missing install-core.ps1.' }
. $core
$source = [IO.Path]::GetFullPath($PSScriptRoot)
$target = Resolve-SovraInstallDirectory $Prefix
$sourcePrefix = $source.TrimEnd('\') + '\'
$targetPrefix = $target.TrimEnd('\') + '\'
if ($targetPrefix.StartsWith($sourcePrefix, [StringComparison]::OrdinalIgnoreCase) -or
    $sourcePrefix.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Install prefix must not overlap the package directory.'
}
foreach ($relative in @('bin\svr.exe','bin\svr-new.ps1','bin\svr-editor.ps1','templates\console\main.svr',
        'developer-tools\launch.mjs','developer-tools\manifest.json','developer-tools\editor\server.mjs',
        'developer-tools\ide\package.json','developer-tools\ide\language-server\server.mjs',
        'templates\console\sovra.toml','docs\spec.md','examples\local-packages\app\main.svr',
        'samples\task-manager\main.svr','README.md','verify-install.ps1',
        'imports.json','integrity.json','install-core.ps1','rollback.ps1','LICENSE','LICENSE-MIT','LICENSE-APACHE',
        'runtime-source\payload.json','runtime-source\payload.sha256','runtime-source\Cargo.toml','runtime-source\Cargo.lock',
        'runtime-source\src\lib.rs','runtime-source\examples\native_builder.rs',
        'runtime-source\toolchain\templates\console\main.svr',
        'runtime-source\toolchain\templates\console\sovra.toml')) {
    if (-not (Test-Path -LiteralPath (Join-Path $source $relative) -PathType Leaf)) {
        throw "Package is missing $relative."
    }
}
Assert-SovraInstallPackage $source
if ((Test-Path -LiteralPath $target) -and -not $Upgrade) { throw "Install prefix already exists: $target" }
$parent = [IO.Path]::GetFullPath((Split-Path -Parent $target)).TrimEnd('\')
New-Item -ItemType Directory -Path $parent -Force | Out-Null
$stage = [IO.Path]::GetFullPath((Join-Path $parent ('.sovra-install-' + [guid]::NewGuid().ToString('N'))))
if (-not $stage.StartsWith($parent + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unsafe installation staging path.'
}
$commitStarted = $false
try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    foreach ($directory in @('bin','templates','samples','docs','examples','runtime-source','developer-tools')) {
        Copy-Item -LiteralPath (Join-Path $source $directory) -Destination $stage -Recurse
    }
    foreach ($file in @('README.md','install.ps1','install-core.ps1','rollback.ps1','verify-install.ps1','imports.json','integrity.json',
            'LICENSE','LICENSE-MIT','LICENSE-APACHE')) {
        Copy-Item -LiteralPath (Join-Path $source $file) -Destination $stage
    }
    $commitStarted = $true
    $result = Invoke-SovraInstallCommit -Stage $stage -Target $target -Upgrade:$Upgrade -Validate { param($path) Test-SovraInstallCompiler $path }
    if ($result.Backup) { Write-Output "Previous installation retained at $($result.Backup)" }
} finally {
    if (Test-Path -LiteralPath $stage) {
        if (-not $commitStarted) { Remove-Item -LiteralPath $stage -Recurse -Force }
        else { Write-Warning "Unpublished installation retained for inspection: $stage" }
    }
}
$bin = Join-Path $target 'bin'
if (-not $NoPersistPath) {
    $current = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($current -split ';' | Where-Object { $_ })
    if (-not ($entries | Where-Object { $_.TrimEnd('\') -ieq $bin.TrimEnd('\') })) {
        [Environment]::SetEnvironmentVariable('Path', $(if ($current) { $current + ';' + $bin } else { $bin }), 'User')
    }
}
$env:Path = "$bin;$env:Path"
Write-Output "Installed Sovra in $target"
Write-Output 'Open a new PowerShell window to use svr and svr-new from PATH.'
