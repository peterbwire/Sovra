param(
    [Parameter(Mandatory=$true)][string]$Package,
    [Parameter(Mandatory=$true)][string]$Screenshot
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($Package)
. (Join-Path $root 'ui\renderer.ps1')
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-window-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch | Out-Null
$result = [pscustomobject]@{ Step = 0; Error = ''; Passed = $false; Offset = 0.0 }
$window = $null
$timer = $null
try {
    $profile = Get-Content -LiteralPath (Join-Path $root 'app.json') -Raw | ConvertFrom-Json
    $viewCommand = $(if ($profile.PSObject.Properties.Name -contains 'view_command') { $profile.view_command } else { 'ui-view' })
    $session = New-SovraUiSession -Program (Join-Path $root 'application.exe') -State (Join-Path $scratch 'state.txt') -ViewCommand $viewCommand
    $window = New-SovraDesktopWindow $session
    if ($viewCommand -ceq 'ui-layout-view' -and -not $session.Groups.ContainsKey('add-task')) { throw 'Grouped package did not open its Sovra cards.' }
    # Build enough real task rows to exercise scroll restoration.
    for ($index = 1; $index -le 12; $index++) {
        $null = Invoke-SovraUiEvent $session 'add' "Existing task $index" -HasValue
    }
    Update-SovraDesktopControls $session
    $timer = [Windows.Threading.DispatcherTimer]::new()
    $timer.Interval = [TimeSpan]::FromSeconds(1)
    $timer.Add_Tick({
        try {
            if ($session.Busy) { return }
            switch ($result.Step) {
                0 {
                    $window.Activate() | Out-Null
                    $session.Inputs['add'].Text = 'Built with Sovra UI'
                    $session.Buttons['add'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
                }
                1 {
                    if (-not $session.Buttons.ContainsKey('complete-13')) { throw ('Native Add failed: ' + $session.Status.Text) }
                    $session.Inputs['add'].Text = 'Keep this unfinished draft'
                    $session.Inputs['add'].Select(5, 8)
                    [void]$session.Inputs['add'].Focus()
                    $session.Scroll.ScrollToVerticalOffset(250)
                    $window.UpdateLayout()
                }
                2 {
                    $result.Offset = $session.Scroll.VerticalOffset
                    if ($result.Offset -le 0) { throw 'Scroll test did not establish an offset.' }
                    $session.Buttons['complete-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
                }
                3 {
                    if (-not @($session.Nodes | Where-Object { $_.Text -match '^Done #1' }).Count) { throw 'Native Complete failed.' }
                    if ($session.Inputs['add'].Text -cne 'Keep this unfinished draft') { throw 'Draft was lost after Complete.' }
                    if ($session.Inputs['add'].SelectionStart -ne 5 -or $session.Inputs['add'].SelectionLength -ne 8) { throw 'Selection was lost after Complete.' }
                    if (-not $session.Inputs['add'].IsKeyboardFocused) { throw 'Keyboard focus was lost after Complete.' }
                    if ([Math]::Abs($session.Scroll.VerticalOffset - $result.Offset) -gt 1) { throw 'Scroll position was lost after layout.' }
                    $window.UpdateLayout()
                    $bitmap = [Windows.Media.Imaging.RenderTargetBitmap]::new(
                        [int]$window.ActualWidth, [int]$window.ActualHeight, 96, 96, [Windows.Media.PixelFormats]::Pbgra32)
                    $bitmap.Render($window)
                    $encoder = [Windows.Media.Imaging.PngBitmapEncoder]::new()
                    $encoder.Frames.Add([Windows.Media.Imaging.BitmapFrame]::Create($bitmap))
                    $path = [IO.Path]::GetFullPath($Screenshot)
                    [void][IO.Directory]::CreateDirectory((Split-Path -Parent $path))
                    $file = [IO.File]::Create($path)
                    try { $encoder.Save($file) } finally { $file.Dispose() }
                }
                4 {
                    [void]$session.Buttons['delete-1'].Focus()
                    $session.Buttons['delete-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
                }
                5 {
                    if ($session.Buttons.ContainsKey('delete-1')) { throw 'Native Delete failed.' }
                    if (-not $session.Inputs['add'].IsKeyboardFocused) { throw 'Focus was not recovered after its button was removed.' }
                    $result.Passed = $true
                    $timer.Stop()
                    $window.Close()
                }
            }
            $result.Step++
        } catch {
            $result.Error = $_.Exception.Message
            $timer.Stop()
            $window.Close()
        }
    }.GetNewClosure())
    $timer.Start()
    [void]$window.ShowDialog()
    if (-not $result.Passed) { throw ('Desktop window test failed: ' + $result.Error) }
    Write-Output "PASS visible desktop window: native events, draft/selection/focus/scroll preserved; screenshot $Screenshot"
} finally {
    if ($timer) { $timer.Stop() }
    if ($window) { $window.Close() }
    $resolved = [IO.Path]::GetFullPath($scratch)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-window-test-'
    if (-not $resolved.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected window test cleanup path.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
