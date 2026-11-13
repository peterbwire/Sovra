param([string]$Report)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '..\toolchain\install-core.ps1')
$checks = [Collections.Generic.List[string]]::new()
$passed = $false
$failure = $null
$work = Join-Path ([IO.Path]::GetTempPath()) ('sovra-upgrade-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
function New-Fixture([string]$Path, [string]$Value) {
    New-Item -ItemType Directory -Path (Join-Path $Path 'bin') -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $Path 'bin/svr.exe'), $Value)
    [IO.File]::WriteAllText((Join-Path $Path 'payload.txt'), $Value)
    $files = @('bin/svr.exe', 'payload.txt') | ForEach-Object { [pscustomobject]@{ path = $_; sha256 = Get-SovraInstallHash (Join-Path $Path $_) } }
    [IO.File]::WriteAllText((Join-Path $Path 'integrity.json'), ([pscustomobject]@{ schema = 1; files = @($files) } | ConvertTo-Json -Depth 4))
}
function New-Stage([string]$Value) {
    $path = Join-Path $work ('.sovra-install-' + [guid]::NewGuid().ToString('N'))
    New-Fixture $path $Value
    return $path
}
function Expect-Rejected([scriptblock]$Action) {
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw 'Expected installation rejection.' }
}
$validate = { param($path) if (-not (Test-Path -LiteralPath (Join-Path $path 'payload.txt'))) { throw 'Missing payload' } }
try {
    $failedTarget = Join-Path $work 'failed initial install'
    $failedStage = New-Stage 'candidate'
    $script:FailTarget = [IO.Path]::GetFullPath($failedTarget)
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $failedStage -Target $failedTarget -Validate { param($path) if ($path -eq $script:FailTarget) { throw 'Injected first-install failure' } } }
    if ((Test-Path -LiteralPath $failedTarget) -or -not (Test-Path -LiteralPath $failedStage)) { throw 'Failed initial activation published a partial target.' }
    $checks.Add('failed-first-activation-retains-candidate')
    $target = Join-Path $work 'installed with spaces'
    if ((Get-SovraInstallLeasePath $target) -cne (Get-SovraInstallLeasePath ($target + [IO.Path]::DirectorySeparatorChar))) { throw 'Trailing separator changed installation lease identity.' }
    Expect-Rejected { Resolve-SovraInstallDirectory ([IO.Path]::GetPathRoot($target)) }
    $stage = New-Stage 'old'
    $first = Invoke-SovraInstallCommit -Stage $stage -Target $target -Validate $validate
    if ($first.Backup) { throw 'Fresh installation created a backup.' }
    $checks.Add('fresh-install')
    $next = New-Stage 'new'
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $next -Target $target -Validate $validate }
    if ([IO.File]::ReadAllText((Join-Path $target 'payload.txt')) -ne 'old') { throw 'Refused reinstall changed files.' }
    $checks.Add('no-overwrite')
    $lease = [IO.File]::Open((Get-SovraInstallLeasePath $target), [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    try { Expect-Rejected { Invoke-SovraInstallCommit -Stage $next -Target $target -Upgrade -Validate $validate } }
    finally { $lease.Dispose() }
    $checks.Add('exclusive-prefix-lease')
    [IO.File]::WriteAllText((Join-Path $target 'payload.txt'), 'user modification')
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $next -Target $target -Upgrade -Validate $validate }
    if ([IO.File]::ReadAllText((Join-Path $target 'payload.txt')) -ne 'user modification') { throw 'Modified installed file changed.' }
    [IO.File]::WriteAllText((Join-Path $target 'payload.txt'), 'old')
    [IO.File]::WriteAllText((Join-Path $target 'personal-data.txt'), 'preserve')
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $next -Target $target -Upgrade -Validate $validate }
    if ([IO.File]::ReadAllText((Join-Path $target 'personal-data.txt')) -ne 'preserve') { throw 'Unmanaged data changed.' }
    $checks.Add('modified-and-foreign-files-preserved')
    Remove-Item -LiteralPath (Join-Path $target 'personal-data.txt')
    [IO.File]::WriteAllText((Join-Path $next 'payload.txt'), 'tampered')
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $next -Target $target -Upgrade -Validate $validate }
    Remove-Item -LiteralPath $next -Recurse -Force
    $checks.Add('tampered-candidate-rejected')
    $next = New-Stage 'new'
    $script:FailTarget = [IO.Path]::GetFullPath($target)
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $next -Target $target -Upgrade -Validate { param($path) if ($path -eq $script:FailTarget) { throw 'Injected activation failure' } } }
    if ([IO.File]::ReadAllText((Join-Path $target 'payload.txt')) -ne 'old' -or -not (Test-Path -LiteralPath $next)) { throw 'Activation failure did not restore old files and retain candidate.' }
    $checks.Add('failed-upgrade-restores-old-and-retains-candidate')
    $result = Invoke-SovraInstallCommit -Stage $next -Target $target -Upgrade -Validate $validate
    if ([IO.File]::ReadAllText((Join-Path $target 'payload.txt')) -ne 'new' -or [IO.File]::ReadAllText((Join-Path $result.Backup 'payload.txt')) -ne 'old') { throw 'Upgrade/backup failed.' }
    $checks.Add('upgrade-retains-old-snapshot')
    Expect-Rejected { Assert-SovraInstallRecoveryReceipt (Join-Path $work 'wrong target') $result.Backup }
    $receiptPath = $result.Backup + '.recovery.json'
    $receiptBytes = [IO.File]::ReadAllBytes($receiptPath)
    $receipt = [IO.File]::ReadAllText($receiptPath) | ConvertFrom-Json
    $receipt.compiler_sha256 = '0' * 64
    [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json))
    Expect-Rejected { Invoke-SovraInstallCommit -Stage $result.Backup -Target $target -Upgrade -Validate $validate }
    if ([IO.File]::ReadAllText((Join-Path $target 'payload.txt')) -ne 'new') { throw 'Rejected recovery changed active installation.' }
    $checks.Add('wrong-target-and-tampered-receipts-rejected')
    [IO.File]::WriteAllBytes($receiptPath, $receiptBytes)
    $restored = Invoke-SovraInstallCommit -Stage $result.Backup -Target $target -Upgrade -Validate $validate
    if ([IO.File]::ReadAllText((Join-Path $target 'payload.txt')) -ne 'old' -or [IO.File]::ReadAllText((Join-Path $restored.Backup 'payload.txt')) -ne 'new') { throw 'Explicit rollback did not retain both snapshots.' }
    $checks.Add('rollback-retains-displaced-snapshot')
    # Simulate a process dying after the old directory was moved aside.
    $recovery = Join-Path $work ('.sovra-backup-' + [guid]::NewGuid().ToString('N'))
    New-SovraInstallRecoveryReceipt $target $recovery
    [IO.Directory]::Move($target, $recovery)
    Invoke-SovraInstallCommit -Stage $recovery -Target $target -Upgrade -Validate $validate | Out-Null
    Assert-SovraInstallPackage $target
    $checks.Add('missing-prefix-recovery')
    $linkTarget = Join-Path $work 'outside'
    New-Item -ItemType Directory -Path $linkTarget | Out-Null
    $link = Join-Path $target 'linked-data'
    if ([IO.Path]::DirectorySeparatorChar -eq '\') { New-Item -ItemType Junction -Path $link -Target $linkTarget | Out-Null }
    else { New-Item -ItemType SymbolicLink -Path $link -Target $linkTarget | Out-Null }
    Expect-Rejected { Assert-SovraInstallPackage $target }
    # Remove only the link, never its target, before fixture cleanup.
    [IO.Directory]::Delete($link)
    $checks.Add('linked-directory-rejected')
    $passed = $true
    Write-Output 'PASS installer transactions: no-overwrite, lease, unmanaged/tampered files, activation rollback, retained backups, crash recovery and link rejection.'
} catch {
    $failure = $_.Exception.Message
    throw
} finally {
    if (Test-Path -LiteralPath $work) { Remove-Item -LiteralPath $work -Recurse -Force }
    if ($Report) {
        $reportPath = [IO.Path]::GetFullPath($Report)
        [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($reportPath)) | Out-Null
        $sources = @('scripts/test-install-transaction.ps1', 'toolchain/install-core.ps1') | ForEach-Object {
            [pscustomobject]@{ path = $_; sha256 = Get-SovraInstallHash (Join-Path $PSScriptRoot ('../' + $_)) }
        }
        $evidence = [ordered]@{
            schema = 1; suite = 'installer-transaction-fixtures'; success = $passed
            recorded_utc = [DateTime]::UtcNow.ToString('o')
            os = [Environment]::OSVersion.VersionString; powershell = $PSVersionTable.PSVersion.ToString()
            sources = @($sources); passed_checks = @($checks.ToArray()); failure = $failure
            real_compiler_executed = $false; clean_windows_qualified = $false
            pending = @('distinct real old/new packages', 'Windows PowerShell 5.1 installer entrypoints', 'persistent user PATH', 'real application profiles', 'clean-machine execution')
        }
        [IO.File]::WriteAllText($reportPath, ($evidence | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
    }
}
