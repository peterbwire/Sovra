param([Parameter(Mandatory=$true, Position=0)][string]$Path)
$ErrorActionPreference = 'Stop'
$compiler = Join-Path $PSScriptRoot 'svr.exe'
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
    throw 'Installed Sovra compiler is missing.'
}
& $compiler new $Path
if ($LASTEXITCODE -ne 0) { throw "svr new failed with exit code $LASTEXITCODE" }
