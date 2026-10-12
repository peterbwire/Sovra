param(
    [string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Sovra'),
    [switch]$NoPersistPath
)
$ErrorActionPreference = 'Stop'
$source = [IO.Path]::GetFullPath($PSScriptRoot)
$target = [IO.Path]::GetFullPath($Prefix)
if ($target.TrimEnd('\') -eq $source.TrimEnd('\')) { throw 'Install prefix must differ from package directory.' }
$binary = Join-Path $source 'bin\svr.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Package is missing bin\svr.exe.' }
if (Test-Path -LiteralPath $target) { throw "Install prefix already exists: $target" }
New-Item -ItemType Directory -Path $target | Out-Null
Copy-Item -LiteralPath (Join-Path $source 'bin') -Destination $target -Recurse
Copy-Item -LiteralPath (Join-Path $source 'templates') -Destination $target -Recurse
Copy-Item -LiteralPath (Join-Path $source 'docs') -Destination $target -Recurse
foreach ($file in @('README.md','LICENSE','LICENSE-MIT','LICENSE-APACHE')) {
    Copy-Item -LiteralPath (Join-Path $source $file) -Destination $target
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
