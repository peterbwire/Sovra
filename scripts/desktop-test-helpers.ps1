function Wait-SovraDesktopIdle($Session, [int]$TimeoutMilliseconds = 15000) {
    if (-not $Session.Busy) { return }
    $frame = [Windows.Threading.DispatcherFrame]::new()
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $state = [pscustomobject]@{ TimedOut = $false }
    $timer = [Windows.Threading.DispatcherTimer]::new()
    $timer.Interval = [TimeSpan]::FromMilliseconds(10)
    $timer.Add_Tick({
        if ($watch.ElapsedMilliseconds -gt $TimeoutMilliseconds) { $state.TimedOut = $true; $frame.Continue = $false }
        elseif (-not $Session.Busy) { $frame.Continue = $false }
    }.GetNewClosure())
    try {
        $timer.Start()
        [Windows.Threading.Dispatcher]::PushFrame($frame)
        if ($state.TimedOut) { throw 'Desktop operation failed to complete within the test deadline.' }
    } finally { $timer.Stop() }
}
