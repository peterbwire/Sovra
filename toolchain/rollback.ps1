param(
    [string]$Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Sovra'),
    [Parameter(Mandatory=$true)][string]$Backup
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-core.ps1')
if ([IO.Path]::GetFileName([IO.Path]::GetFullPath($Backup)) -cnotmatch '^\.sovra-backup-[0-9a-f]{32}$') { throw 'Rollback requires a retained Sovra backup directory.' }
$result = Invoke-SovraInstallCommit -Stage $Backup -Target $Prefix -Upgrade -Validate { param($path) Test-SovraInstallCompiler $path }
Write-Output "Restored Sovra in $($result.Target)"
if ($result.Backup) { Write-Output "Previous installation retained at $($result.Backup)" }
