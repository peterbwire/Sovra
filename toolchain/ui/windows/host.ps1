param(
    [string]$Program = '',
    [string]$State = '',
    [string]$Source = '',
    [string]$ApplicationProfile = '',
    [ValidateSet('ui-view', 'ui-layout-view')][string]$ViewCommand = 'ui-view'
)
$ErrorActionPreference = 'Stop'
try {
    . (Join-Path $PSScriptRoot 'renderer.ps1')
    if ($ApplicationProfile) {
        if ($Program -or $State -or $Source) { throw 'Use a profile or explicit program/state, not both.' }
        if ((Get-Item -LiteralPath $ApplicationProfile).Length -gt 16384) { throw 'UI application profile is too large.' }
        $config = [IO.File]::ReadAllText([IO.Path]::GetFullPath($ApplicationProfile)) | ConvertFrom-Json
        if ($config.schema -ne 1 -or $config.id -cnotmatch '^[a-z][a-z0-9-]{0,63}$' -or
            $config.program -cne 'application.exe') { throw 'Invalid private UI application profile.' }
        if ($config.PSObject.Properties.Name -contains 'view_command') {
            if ($config.view_command -cnotin @('ui-view', 'ui-layout-view')) { throw 'Invalid UI view command.' }
            $ViewCommand = $config.view_command
        }
        $Program = Join-Path (Split-Path -Parent ([IO.Path]::GetFullPath($ApplicationProfile))) $config.program
        $dataRoot = [Environment]::GetFolderPath('LocalApplicationData')
        if (-not $dataRoot) { throw 'Windows local application data is unavailable.' }
        $directory = Join-Path $dataRoot ('Sovra\Desktop\' + $config.id)
        [void][IO.Directory]::CreateDirectory($directory)
        $State = Join-Path $directory 'state.txt'
    }
    if (-not $Program -or -not $State) { throw 'A Sovra program and state path are required.' }
    $session = New-SovraUiSession -Program $Program -State $State -Source $Source -ViewCommand $ViewCommand
    $window = New-SovraDesktopWindow $session
    [void]$window.ShowDialog()
} catch {
    Add-Type -AssemblyName PresentationFramework
    [void][Windows.MessageBox]::Show($_.Exception.Message, 'Sovra UI could not start', 'OK', 'Error')
    # The launcher recognizes this private code as an error already shown to the user.
    exit 10
}
