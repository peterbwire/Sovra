param([Parameter(Mandatory=$true)][string]$Package)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-ui-launch-' + [guid]::NewGuid().ToString('N'))
$appId = 'ui-startup-' + [guid]::NewGuid().ToString('N')
$dataRoot = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Sovra\Desktop'
$ownedData = Join-Path $dataRoot $appId
$launcher = $null
$hostProcess = $null
$originalPath = $env:Path
function Wait-DesktopWindow([Diagnostics.Process]$Process, [string]$Title) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt 20000) {
        if ($Process.HasExited) { throw "Desktop process exited before a visible window: $($Process.ExitCode)" }
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            $element = [Windows.Automation.AutomationElement]::FromHandle($Process.MainWindowHandle)
            if ($element.Current.Name -eq $Title) { return $element }
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Desktop window did not appear: $Title"
}
function Close-DesktopWindow($Element) {
    $pattern = $Element.GetCurrentPattern([Windows.Automation.WindowPattern]::Pattern)
    $pattern.Close()
}
try {
    New-Item -ItemType Directory -Path $scratch | Out-Null
    $app = Join-Path $scratch 'desktop app with spaces and !'
    Copy-Item -LiteralPath ([IO.Path]::GetFullPath($Package)) -Destination $app -Recurse
    [IO.File]::WriteAllText((Join-Path $app 'app.json'),
        (@{ schema = 1; id = $appId; program = 'application.exe' } | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
    $env:Path = "$env:SystemRoot\System32;$env:SystemRoot"
    $start = [Diagnostics.ProcessStartInfo]::new((Join-Path $app 'Sovra UI.exe'))
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $launcher = [Diagnostics.Process]::Start($start)
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt 20000 -and -not $hostProcess) {
        $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $($launcher.Id)" |
            Where-Object { $_.Name -ieq 'powershell.exe' })
        if ($children.Count -eq 1) { $hostProcess = [Diagnostics.Process]::GetProcessById($children[0].ProcessId) }
        else { Start-Sleep -Milliseconds 100 }
    }
    if (-not $hostProcess) { throw 'Desktop EXE did not launch its host.' }
    $window = Wait-DesktopWindow $hostProcess 'Sovra Task Manager'
    $inputElement = $window.FindFirst([Windows.Automation.TreeScope]::Descendants,
        [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty, 'Task title'))
    if (-not $inputElement) { throw 'Packaged desktop window is missing the Sovra input.' }
    Close-DesktopWindow $window
    if (-not $launcher.WaitForExit(10000) -or $launcher.ExitCode -ne 0) { throw 'Desktop launcher did not exit cleanly.' }
    $hostProcess = $null
    $launcher.Dispose()
    $launcher = $null
    # A missing host used to make a hidden launcher exit silently. Check its dialog.
    Move-Item -LiteralPath (Join-Path $app 'ui\host.ps1') -Destination (Join-Path $app 'ui\host.missing')
    $launcher = [Diagnostics.Process]::Start($start)
    $errorWindow = Wait-DesktopWindow $launcher 'Sovra UI could not start'
    Close-DesktopWindow $errorWindow
    if (-not $launcher.WaitForExit(10000) -or $launcher.ExitCode -ne 1) { throw 'Startup failure was not propagated.' }
    Write-Output 'PASS packaged desktop EXE: visible native window, restricted PATH, spaced path and startup-error dialog.'
} finally {
    $env:Path = $originalPath
    foreach ($process in @($hostProcess, $launcher)) {
        if ($process) {
            if (-not $process.HasExited) { $process.Kill(); [void]$process.WaitForExit(5000) }
            $process.Dispose()
        }
    }
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-ui-launch-'
    if (-not [IO.Path]::GetFullPath($scratch).StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected launcher test path.' }
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
    if ([IO.Path]::GetFullPath($ownedData) -cne ([IO.Path]::GetFullPath($dataRoot).TrimEnd('\') + '\' + $appId)) { throw 'Unexpected launcher test data path.' }
    if (Test-Path -LiteralPath $ownedData) { Remove-Item -LiteralPath $ownedData -Recurse -Force }
}
