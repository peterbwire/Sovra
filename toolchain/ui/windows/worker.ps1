# Run only application work here. No WPF objects cross into this runspace.
param(
    [string]$Program, [string]$Source, [string]$State, [string]$Action,
    [AllowEmptyString()][string]$Value, [bool]$HasValue, [int]$TimeoutMilliseconds,
    [ValidateSet('ui-view', 'ui-layout-view')][string]$ViewCommand = 'ui-view'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'renderer.ps1')
$session = $null
$statusCode = $null
$errorText = ''
try {
    $session = New-SovraUiSession -Program $Program -Source $Source -State $State -ViewCommand $ViewCommand
    $session.TimeoutMilliseconds = $TimeoutMilliseconds
    $statusCode = Invoke-SovraUiEvent $session $Action $Value -HasValue:$HasValue
} catch { $errorText = $_.Exception.Message }
[pscustomobject]@{
    Status = $statusCode; Error = $errorText
    Nodes = $(if ($session) { @($session.Nodes) } else { @() })
    Notice = $(if ($session) { $session.Notice } else { '' })
}
