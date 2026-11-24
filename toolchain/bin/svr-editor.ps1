param([string]$NodePath = 'node', [switch]$Check)
$ErrorActionPreference = 'Stop'
$launcher = Join-Path (Split-Path -Parent $PSScriptRoot) 'developer-tools\launch.mjs'
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw 'Installed Sovra developer tools are missing.' }
$node = Get-Command $NodePath -CommandType Application -ErrorAction Stop
$mode = 'editor'
if ($Check) { $mode = '--check' }
& $node.Source $launcher $mode
if ($LASTEXITCODE -ne 0) { throw "Sovra editor failed with exit code $LASTEXITCODE" }
