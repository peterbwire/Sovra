param(
    [Parameter(Mandatory=$true)][string]$Prefix,
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
& (Join-Path $Prefix 'verify-install.ps1') -Prefix $Prefix | Out-Null
foreach ($target in @('js','exe')) {
    $json = (& svr.exe doctor --target $target --format json) -join "`n"
    if ($LASTEXITCODE -ne 1) { throw "Doctor accepted missing $target prerequisites on restricted PATH." }
    $health = $json | ConvertFrom-Json
    if ($health.success -ne $false -or @($health.checks | Where-Object { $_.required -and $_.status -eq 'fail' }).Count -eq 0) {
        throw "Doctor did not explain missing $target prerequisites."
    }
}
$version = & svr.exe --version
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^svr [0-9]+\.') { throw 'Installed compiler version failed.' }
$taskDir = Join-Path (Split-Path -Parent $Prefix) 'task-manager-checkout'
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
Write-Output "Installed toolchain passed in restricted PATH: $version; package self-test and Task Manager."
