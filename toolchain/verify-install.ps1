param([string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Sovra'))
$ErrorActionPreference = 'Stop'
$installed = [IO.Path]::GetFullPath($Prefix)
$bin = Join-Path $installed 'bin'
$compiler = Join-Path $bin 'svr.exe'
$creator = Join-Path $bin 'svr-new.ps1'
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) { throw "Installed compiler missing: $compiler" }
if (-not (Test-Path -LiteralPath $creator -PathType Leaf)) { throw "Project creator missing: $creator" }
$resolved = @(Get-Command svr.exe -CommandType Application -ErrorAction Stop)[0].Source
if (-not [string]::Equals([IO.Path]::GetFullPath($resolved), $compiler, [StringComparison]::OrdinalIgnoreCase)) {
    throw "PATH resolves another compiler: $resolved"
}
$version = & svr.exe --version
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^svr [0-9]+\.') { throw 'Installed compiler version failed.' }
$doctor = (& svr.exe doctor --format json) -join "`n"
if ($LASTEXITCODE -ne 0) { throw 'Installed interpreter doctor failed.' }
$health = $doctor | ConvertFrom-Json
if ($health.schema_version -ne 1 -or $health.kind -ne 'doctor' -or $health.success -ne $true -or
    @($health.checks | Where-Object { $_.id -eq 'install.integrity' -and $_.status -eq 'pass' -and $_.required }).Count -ne 1) {
    throw 'Installed doctor did not validate package integrity.'
}
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("sovra-install-check-" + [guid]::NewGuid().ToString('N'))
$project = Join-Path $testRoot 'hello'
New-Item -ItemType Directory -Path $testRoot | Out-Null
try {
    & svr.exe new $project | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Installed svr new failed.' }
    $manifest = Get-Content -LiteralPath (Join-Path $project 'sovra.toml') -Raw
    if ($manifest -notmatch 'name = "hello"') { throw 'Generated project manifest name mismatch.' }
    $generatedSource = Get-Content -LiteralPath (Join-Path $project 'main.svr') -Raw
    $templateSource = Get-Content -LiteralPath (Join-Path $installed 'templates\console\main.svr') -Raw
    if ($generatedSource -ne $templateSource) { throw 'Embedded console template differs from packaged template.' }
    $formatSource = Join-Path $testRoot 'format smoke.svr'
    [IO.File]::WriteAllText($formatSource, 'fn main(){print("format smoke");}', (New-Object Text.UTF8Encoding($false)))
    & svr.exe fmt --write $formatSource
    if ($LASTEXITCODE -ne 0) { throw 'Installed formatter write failed.' }
    & svr.exe fmt --check $formatSource
    if ($LASTEXITCODE -ne 0) { throw 'Installed formatter check failed.' }
    $formatResult = & svr.exe run $formatSource
    if ($LASTEXITCODE -ne 0 -or $formatResult -ne 'format smoke') { throw 'Installed formatted program failed.' }
    $wrapperProject = Join-Path $testRoot 'wrapper'
    & $creator $wrapperProject | Out-Null
    if (-not (Test-Path -LiteralPath (Join-Path $wrapperProject 'main.svr'))) {
        throw 'Compatibility project creator failed.'
    }
    $rejected = $false
    try { & $creator $project *> $null } catch { $rejected = $true }
    if (-not $rejected) { throw 'Project creator overwrote an existing project.' }
    & svr.exe check (Join-Path $project 'main.svr') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Installed project check failed.' }
    $result = & svr.exe run (Join-Path $project 'main.svr')
    if ($LASTEXITCODE -ne 0 -or $result -ne 'Hello, Sovra!') { throw "Installed project run failed: $result" }
    $directoryResult = & svr.exe run $project
    if ($LASTEXITCODE -ne 0 -or $directoryResult -ne 'Hello, Sovra!') {
        throw "Installed project directory run failed: $directoryResult"
    }
    $testReport = (& svr.exe test --format json $project) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Installed project language tests failed.' }
    $tests = $testReport | ConvertFrom-Json
    if ($tests.schema_version -ne 1 -or $tests.success -ne $true -or
        $tests.total -ne 1 -or $tests.tests[0].name -ne 'test_greeting') {
        throw 'Installed project test report is invalid.'
    }
    $packageExample = Join-Path $installed 'examples\local-packages\app'
    & svr.exe check $packageExample | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Installed local-package example check failed.' }
    $packageResult = & svr.exe run $packageExample
    if ($LASTEXITCODE -ne 0 -or ($packageResult -join ',') -ne '49,25,1.5') {
        throw "Installed local-package example run failed: $packageResult"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $installed 'docs\reference\check-json.md'))) {
        throw 'Structured diagnostic documentation missing.'
    }
    $taskDir = Join-Path $testRoot 'task-manager'
    New-Item -ItemType Directory -Path $taskDir | Out-Null
    $taskProgram = Join-Path $taskDir 'main.svr'
    Copy-Item -LiteralPath (Join-Path $installed 'samples\task-manager\main.svr') -Destination $taskProgram
    $state = Join-Path $taskDir 'tasks.txt'
    & svr.exe check $taskProgram | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Bundled Task Manager check failed.' }
    $added = & svr.exe run $taskProgram -- add $state 'Install proof'
    if ($LASTEXITCODE -ne 0 -or $added -ne 'added 1') { throw "Bundled Task Manager add failed: $added" }
    $listed = & svr.exe run $taskProgram -- list $state
    if ($LASTEXITCODE -ne 0 -or ($listed -join "`n") -notmatch '1 \| open \| Install proof') {
        throw "Bundled Task Manager list failed: $listed"
    }
    $beforeBusy = [Convert]::ToBase64String([IO.File]::ReadAllBytes($state))
    $marker = "$state.svr-write-lock"
    [IO.File]::WriteAllText($marker, 'owned bundled self-test marker', (New-Object Text.UTF8Encoding($false)))
    try {
        $blocked = & svr.exe run $taskProgram -- add $state 'Blocked install proof'
        if ($LASTEXITCODE -ne 1 -or $blocked -ne 'error: save busy') {
            throw "Bundled Task Manager busy write was not rejected: $blocked"
        }
        if ([Convert]::ToBase64String([IO.File]::ReadAllBytes($state)) -cne $beforeBusy -or
            [IO.File]::ReadAllText($marker) -cne 'owned bundled self-test marker') {
            throw 'Bundled busy write changed state or another owner marker.'
        }
    } finally {
        # Only this self-test's marker is removed after its invocation has exited.
        Remove-Item -LiteralPath $marker
    }
    $completed = & svr.exe run $taskProgram -- complete $state 'Install proof'
    if ($LASTEXITCODE -ne 0 -or $completed -ne 'completed 1') {
        throw "Bundled Task Manager complete failed: $completed"
    }
    $deleted = & svr.exe run $taskProgram -- delete $state 'Install proof'
    if ($LASTEXITCODE -ne 0 -or $deleted -ne 'deleted 1') {
        throw "Bundled Task Manager delete failed: $deleted"
    }
    Write-Output "Verified $version from $installed; project, local package and Task Manager passed."
} finally {
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    $checkRoot = [IO.Path]::GetFullPath($testRoot)
    if (-not $checkRoot.StartsWith($tempRoot + '\sovra-install-check-', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove an unexpected test path.'
    }
    if (Test-Path -LiteralPath $checkRoot) { Remove-Item -LiteralPath $checkRoot -Recurse -Force }
}
