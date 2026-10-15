param(
    [Parameter(Mandatory=$true)][string]$Package,
    [Parameter(Mandatory=$true)][string]$Screenshot
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($Package)
. (Join-Path $root 'ui\renderer.ps1')
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-window-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch | Out-Null
$result = [pscustomobject]@{ Step = 0; Error = ''; Passed = $false }
$window = $null
$timer = $null
try {
    $session = New-SovraUiSession -Program (Join-Path $root 'application.exe') -State (Join-Path $scratch 'state.txt')
    $window = New-SovraDesktopWindow $session
    $timer = [Windows.Threading.DispatcherTimer]::new()
    $timer.Interval = [TimeSpan]::FromSeconds(1)
    $timer.Add_Tick({
        try {
            switch ($result.Step) {
                0 {
                    $session.Inputs['add'].Text = 'Built with Sovra UI'
                    $session.Buttons['add'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
                    if (-not $session.Buttons.ContainsKey('complete-1')) { throw ('Native Add failed: ' + $session.Status.Text) }
                }
                1 {
                    $session.Buttons['complete-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
                    if (-not @($session.Nodes | Where-Object { $_.Text -match '^Done #1' }).Count) { throw 'Native Complete failed.' }
                }
                2 {
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
                3 {
                    $session.Buttons['delete-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
                    if ($session.Buttons.ContainsKey('delete-1')) { throw 'Native Delete failed.' }
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
    Write-Output "PASS visible desktop window: native Add/Complete/Delete with Sovra; screenshot $Screenshot"
} finally {
    if ($timer) { $timer.Stop() }
    if ($window) { $window.Close() }
    $resolved = [IO.Path]::GetFullPath($scratch)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-window-test-'
    if (-not $resolved.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected window test cleanup path.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
