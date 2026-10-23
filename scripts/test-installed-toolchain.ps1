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
$beforeBusy = [IO.File]::ReadAllBytes($state)
$marker = "$state.svr-write-lock"
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText($marker, 'owned installation test marker', $utf8)
try {
    $blocked = & svr.exe run $taskProgram -- add $state 'Blocked task'
    if ($LASTEXITCODE -ne 1 -or $blocked -ne 'error: save busy') { throw "Installed busy save was not rejected: $blocked" }
    if ([Convert]::ToBase64String([IO.File]::ReadAllBytes($state)) -cne [Convert]::ToBase64String($beforeBusy)) {
        throw 'Installed busy save changed task bytes.'
    }
    if ([IO.File]::ReadAllText($marker) -cne 'owned installation test marker') {
        throw 'Installed busy save removed or changed another owner marker.'
    }
} finally {
    # This test created the marker; its owned invocations have already exited.
    Remove-Item -LiteralPath $marker
}
$recovered = & svr.exe run $taskProgram -- add $state 'Recovered task'
if ($LASTEXITCODE -ne 0 -or $recovered -ne 'added 2') { throw "Installed save did not recover after owned-marker removal: $recovered" }
$conditionalSource = Join-Path $taskDir 'conditional.svr'
$conditionalState = Join-Path $taskDir 'conditional.txt'
[IO.File]::WriteAllText($conditionalSource, @'
fn main() {
    let path = std::arg(0);
    let created: std::TextWrite = std::write_text_if(path, false, "", "base");
    print(created.ok);
    let conflict = std::write_text_if(path, true, "stale", "lost");
    print(conflict.error);
    let changed = std::write_text_if(path, true, "base", "updated");
    print(changed.ok);
    let invalid = std::write_text_if(path, false, "nonempty", "lost");
    print(invalid.error);
}
'@, $utf8)
$conditional = (& svr.exe run $conditionalSource -- $conditionalState) -join "`n"
if ($LASTEXITCODE -ne 0 -or $conditional -cne "true`nconflict`ntrue`ninvalid_snapshot") {
    throw "Installed conditional-write contract failed: $conditional"
}
if ([IO.File]::ReadAllText($conditionalState) -cne 'updated' -or (Test-Path -LiteralPath "$conditionalState.svr-write-lock")) {
    throw 'Installed conditional replacement lost data or left its normal-completion marker.'
}
Write-Output "Installed toolchain passed in restricted PATH: $version; package self-test, conditional replacement and Task Manager."
