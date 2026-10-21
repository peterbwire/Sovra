param([string]$Compiler = 'target\release\svr.exe', [string]$Renderer = '')
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Renderer) { $Renderer = Join-Path $repository 'toolchain\ui\windows\renderer.ps1' }
. $Renderer
. (Join-Path $PSScriptRoot 'desktop-test-helpers.ps1')
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-responsive-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch | Out-Null
$window = $null
$session = $null
try {
    $source = Join-Path $scratch 'slow.svr'
    [IO.File]::WriteAllText($source,
        'fn main() { if std::arg(0) == "ui-view" { print("SVR-UI-1"); print("heading|Responsive component"); print("button|wait|Wait"); } else { while true {} } }', [Text.UTF8Encoding]::new($false))
    $session = New-SovraUiSession -Program ([IO.Path]::GetFullPath($Compiler)) -Source $source -State (Join-Path $scratch 'state.txt')
    $session.TimeoutMilliseconds = 1000
    $window = New-SovraDesktopWindow $session
    $rendererPath = $session.RendererPath
    $session.RendererPath = Join-Path $scratch 'missing\renderer.ps1'
    if (Start-SovraDesktopEvent $session 'wait') { throw 'A missing worker was accepted.' }
    if ($session.Busy -or -not $session.Panel.IsEnabled -or $session.Status.Text -notmatch 'worker is missing') { throw 'Worker startup failure left the UI locked or silent.' }
    $session.RendererPath = $rendererPath
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $session.Buttons['wait'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    if ($clock.ElapsedMilliseconds -gt 500) { throw 'A slow application event blocked the UI thread.' }
    if (-not $session.Busy -or $session.Status.Text -ne 'Working...') { throw ('Worker did not start: ' + $session.Status.Text) }
    if (-not $window.IsEnabled -or $session.Panel.IsEnabled) { throw 'Busy UI did not keep its window responsive and actions disabled.' }
    if (Start-SovraDesktopEvent $session 'wait') { throw 'An overlapping event was accepted.' }
    $heartbeat = [pscustomobject]@{ Count = 0 }
    $pulse = [Windows.Threading.DispatcherTimer]::new()
    $pulse.Interval = [TimeSpan]::FromMilliseconds(15)
    $pulse.Add_Tick({ $heartbeat.Count++ }.GetNewClosure())
    $pulse.Start()
    try { Wait-SovraDesktopIdle $session } finally { $pulse.Stop() }
    if ($heartbeat.Count -lt 5) { throw 'Dispatcher stopped responding while the worker ran.' }
    if ($session.Status.Text -notmatch 'timed out') { throw ('Worker failure was not visible: ' + $session.Status.Text) }
    if ($session.Operation -or $session.OperationTimer -or -not $session.Panel.IsEnabled) { throw 'Worker resources or busy UI were not released.' }
    # A close during failure must wait and then keep the error visible.
    $null = Start-SovraDesktopEvent $session 'wait'
    $window.Close()
    if (-not $session.CloseRequested -or $session.Closed) { throw 'Closing interrupted an in-flight operation.' }
    Wait-SovraDesktopIdle $session
    if ($session.Closed -or $session.CloseRequested) { throw 'Close hid a failed operation.' }
    # A successful event completes exactly once before a pending close.
    $successSource = Join-Path $scratch 'success.svr'
    [IO.File]::WriteAllText($successSource,
        'fn main() { if std::arg(0) == "ui-view" { print("SVR-UI-1"); print("heading|Close component"); print("button|save|Save"); } else { let old = std::read_text(std::arg(1)); let result = std::write_text(std::arg(1), old.text + "committed once"); if result.ok == false { std::set_exit_code(1); } } }', [Text.UTF8Encoding]::new($false))
    $session.Source = $successSource
    $session.TimeoutMilliseconds = 3000
    $null = Start-SovraDesktopEvent $session 'save'
    $window.Close()
    Wait-SovraDesktopIdle $session
    if (-not $session.Closed -or $session.LastOperationStatus -ne 0) { throw 'Successful pending close did not finish.' }
    if (Start-SovraDesktopEvent $session 'save') { throw 'A closed window accepted an event.' }
    if ([IO.File]::ReadAllText((Join-Path $scratch 'state.txt')) -ne 'committed once') { throw 'Close interrupted persistence.' }
    Write-Output 'PASS responsive desktop: dispatcher heartbeat, single event, timeout cleanup, visible failure and graceful close.'
} finally {
    if ($session -and $session.Busy) { Wait-SovraDesktopIdle $session }
    if ($window) { $window.Close() }
    $resolved = [IO.Path]::GetFullPath($scratch)
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-responsive-'
    if (-not $resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected responsive test path.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
