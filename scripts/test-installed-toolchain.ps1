param(
    [Parameter(Mandatory=$true)][string]$Prefix,
    [Parameter(Mandatory=$true)][string]$Project,
    [Parameter(Mandatory=$true)][string]$TaskSource
)
$ErrorActionPreference = 'Stop'
$expectedBin = Join-Path ([IO.Path]::GetFullPath($Prefix)) 'bin'
$command = @(Get-Command svr.exe -CommandType Application -ErrorAction Stop)[0]
if (-not $command.Source.StartsWith($expectedBin, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Wrong compiler resolved: $($command.Source)"
}
foreach ($name in @('node.exe','cargo.exe','rustc.exe')) {
    if (Get-Command $name -ErrorAction SilentlyContinue) { throw "Test PATH still exposes $name" }
}
$version = & svr.exe --version
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^svr [0-9]+\.') { throw 'Installed compiler version failed.' }
& (Join-Path $expectedBin 'svr-new.ps1') $Project | Out-Null
$rejected = $false
try { & (Join-Path $expectedBin 'svr-new.ps1') $Project | Out-Null } catch { $rejected = $true }
if (-not $rejected) { throw 'Project creator overwrote an existing project.' }
$manifest = Get-Content -LiteralPath (Join-Path $Project 'sovra.toml') -Raw
if ($manifest -notmatch 'name = "hello"') { throw 'Project manifest name mismatch.' }
& svr.exe check (Join-Path $Project 'main.svr') | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Installed compiler check failed.' }
$output = & svr.exe run (Join-Path $Project 'main.svr')
if ($LASTEXITCODE -ne 0 -or $output -ne 'Hello, Sovra!') { throw "Installed application failed: $output" }

$taskDir = Join-Path (Split-Path -Parent $Project) 'task-manager-checkout'
New-Item -ItemType Directory -Path $taskDir | Out-Null
$taskProgram = Join-Path $taskDir 'main.svr'
Copy-Item -LiteralPath $TaskSource -Destination $taskProgram
$state = Join-Path $taskDir 'tasks.txt'
& svr.exe check $taskProgram | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Independent Task Manager check failed.' }
$added = & svr.exe run $taskProgram -- add $state 'Independent task'
if ($LASTEXITCODE -ne 0 -or $added -ne 'added 1') { throw "Task Manager add failed: $added" }
$listed = & svr.exe run $taskProgram -- list $state
if ($LASTEXITCODE -ne 0 -or ($listed -join "`n") -notmatch '1 \| open \| Independent task') {
    throw "Task Manager list failed: $listed"
}
$done = & svr.exe run $taskProgram -- complete $state 'Independent task'
if ($LASTEXITCODE -ne 0 -or $done -ne 'completed 1') { throw "Task Manager complete failed: $done" }
$deleted = & svr.exe run $taskProgram -- delete $state 'Independent task'
if ($LASTEXITCODE -ne 0 -or $deleted -ne 'deleted 1') { throw "Task Manager delete failed: $deleted" }
if (-not (Test-Path -LiteralPath (Join-Path $Prefix 'docs\spec.md'))) { throw 'Documentation missing.' }
Write-Output "Installed toolchain passed in restricted PATH: $version; template and Task Manager."
