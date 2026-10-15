# Private Windows renderer for the existing Sovra view/event prototype.
# Dot-source for conformance tests; no windows are created on import.
Set-StrictMode -Version Latest

function ConvertTo-SovraWindowsArgument([string]$Value) {
    if ($Value.Contains([string][char]0)) { throw 'A process argument contains NUL.' }
    $result = [Text.StringBuilder]::new()
    [void]$result.Append('"')
    $slashes = 0
    foreach ($character in $Value.ToCharArray()) {
        if ($character -eq '\') { $slashes++; continue }
        if ($character -eq '"') {
            [void]$result.Append(('\' * (2 * $slashes + 1)))
        } else { [void]$result.Append(('\' * $slashes)) }
        [void]$result.Append($character)
        $slashes = 0
    }
    [void]$result.Append(('\' * (2 * $slashes)))
    [void]$result.Append('"')
    return $result.ToString()
}

function Invoke-SovraUiProgram {
    param($Session, [string[]]$Arguments)
    $argv = @()
    if ($Session.Source) { $argv += @('run', $Session.Source, '--') }
    $argv += $Arguments
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Session.Program
    $start.Arguments = (@($argv | ForEach-Object { ConvertTo-SovraWindowsArgument $_ }) -join ' ')
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.RedirectStandardInput = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    $readers = @()
    $started = $false
    try {
        if (-not $process.Start()) { throw 'Could not start Sovra application.' }
        $started = $true
        $process.StandardInput.Close()
        foreach ($stream in @($process.StandardOutput.BaseStream, $process.StandardError.BaseStream)) {
            $buffer = [byte[]]::new(4096)
            $readers += [pscustomobject]@{
                Stream = $stream; Buffer = $buffer; Bytes = [IO.MemoryStream]::new()
                Pending = $stream.ReadAsync($buffer, 0, $buffer.Length); Done = $false
            }
        }
        $clock = [Diagnostics.Stopwatch]::StartNew()
        while (@($readers | Where-Object { -not $_.Done }).Count -gt 0 -or -not $process.HasExited) {
            if ($clock.ElapsedMilliseconds -gt $Session.TimeoutMilliseconds) { throw 'Sovra UI operation timed out.' }
            foreach ($reader in $readers) {
                if (-not $reader.Done -and $reader.Pending.IsCompleted) {
                    $count = $reader.Pending.GetAwaiter().GetResult()
                    if ($count -eq 0) { $reader.Done = $true; continue }
                    if ($reader.Bytes.Length + $count -gt 65536) { throw 'Sovra UI output exceeds 65536 bytes.' }
                    $reader.Bytes.Write($reader.Buffer, 0, $count)
                    $reader.Pending = $reader.Stream.ReadAsync($reader.Buffer, 0, $reader.Buffer.Length)
                }
            }
            if (@($readers | Where-Object { -not $_.Done }).Count -gt 0 -or -not $process.HasExited) {
                Start-Sleep -Milliseconds 10
            }
        }
        $utf8 = [Text.UTF8Encoding]::new($false, $true)
        return [pscustomobject]@{
            Status = $process.ExitCode
            Output = $utf8.GetString($readers[0].Bytes.ToArray()).Replace("`r`n", "`n")
            Error = $utf8.GetString($readers[1].Bytes.ToArray()).Replace("`r`n", "`n")
        }
    } finally {
        if ($started -and -not $process.HasExited) {
            $process.Kill()
            [void]$process.WaitForExit(5000)
        }
        foreach ($reader in $readers) { $reader.Bytes.Dispose() }
        $process.Dispose()
    }
}

function ConvertFrom-SovraUiView([string]$Text) {
    if ([Text.Encoding]::UTF8.GetByteCount($Text) -gt 65536) { throw 'UI view exceeds 65536 bytes.' }
    if (-not $Text.EndsWith("`n")) { throw 'Incomplete UI view.' }
    $lines = $Text.Substring(0, $Text.Length - 1).Split([char]"`n")
    if ($lines[0] -cne 'SVR-UI-1') { throw 'Unsupported UI view version.' }
    if ($lines.Count -lt 2 -or $lines.Count -gt 257) { throw 'UI view must have 1 to 256 nodes.' }
    $nodes = @()
    foreach ($line in $lines[1..($lines.Count - 1)]) {
        $split = $line.IndexOf('|')
        if ($split -lt 0) { throw 'Invalid UI node.' }
        $kind = $line.Substring(0, $split)
        $value = $line.Substring($split + 1)
        $node = [pscustomobject]@{ Kind = $kind; Text = ''; Action = ''; Label = ''; Submit = '' }
        if ($kind -ceq 'heading' -or $kind -ceq 'text') {
            if (-not $value -or $value -match '[\x00-\x1f\x7f-\x9f]') { throw 'Invalid UI text.' }
            $node.Text = $value
        } elseif ($kind -ceq 'button' -or $kind -ceq 'input') {
            $first = $value.IndexOf('|')
            if ($first -lt 0) { throw 'Invalid UI action.' }
            $node.Action = $value.Substring(0, $first)
            if ($node.Action -cnotmatch '^[a-z][a-z0-9-]{0,63}$') { throw 'Invalid UI action key.' }
            if ($kind -ceq 'button') { $node.Label = $value.Substring($first + 1) }
            else {
                $last = $value.LastIndexOf('|')
                if ($first -eq $last) { throw 'Invalid UI input.' }
                $node.Label = $value.Substring($first + 1, $last - $first - 1)
                $node.Submit = $value.Substring($last + 1)
                if (-not $node.Submit) { throw 'Missing UI input submit label.' }
            }
            if (-not $node.Label -or ($node.Label + $node.Submit) -match '[\x00-\x1f\x7f-\x9f]') {
                throw 'Invalid UI action label.'
            }
        } else { throw 'Unsupported UI node.' }
        $nodes += $node
    }
    if ($nodes[0].Kind -cne 'heading') { throw 'Missing UI heading.' }
    return $nodes
}

function New-SovraUiSession {
    param([string]$Program, [string]$State, [string]$Source = '')
    if (-not (Test-Path -LiteralPath $Program -PathType Leaf)) { throw 'Sovra program was not found.' }
    if ($Source -and -not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw 'Sovra source was not found.' }
    return [pscustomobject]@{
        Program = [IO.Path]::GetFullPath($Program); State = [IO.Path]::GetFullPath($State)
        Source = $(if ($Source) { [IO.Path]::GetFullPath($Source) } else { '' })
        Nodes = @(); Notice = ''; Busy = $false; Window = $null; Panel = $null
        Status = $null; Buttons = @{}; Inputs = @{}; TimeoutMilliseconds = 30000
    }
}

function Update-SovraUiView($Session) {
    $view = Invoke-SovraUiProgram $Session @('ui-view', $Session.State)
    if ($view.Status -ne 0) { throw ('Cannot load Sovra view: ' + $view.Output + $view.Error) }
    $Session.Nodes = @(ConvertFrom-SovraUiView $view.Output)
}

function Invoke-SovraUiEvent {
    param($Session, [string]$Action, [AllowEmptyString()][string]$Value = '', [switch]$HasValue)
    # Re-check the latest view before dispatch; never trust stale widget action IDs.
    Update-SovraUiView $Session
    $matchingNodes = @($Session.Nodes | Where-Object { $_.Action -ceq $Action -and $_.Kind -cin @('input', 'button') })
    if ($matchingNodes.Count -ne 1) { throw 'UI action is unavailable or ambiguous.' }
    $node = $matchingNodes[0]
    if (($node.Kind -ceq 'input') -ne [bool]$HasValue) { throw 'UI event value does not match its control.' }
    if ([Text.Encoding]::UTF8.GetByteCount($Value) -gt 65536) { throw 'UI event value exceeds 65536 bytes.' }
    $argsList = @('ui-event', $Session.State, $Action)
    if ($HasValue) { $argsList += $Value }
    $result = Invoke-SovraUiProgram $Session $argsList
    $Session.Notice = ($result.Output + $result.Error).Trim()
    if ($result.Status -ne 0) {
        $Session.Notice = "Operation failed (exit $($result.Status)). " + $Session.Notice
    }
    Update-SovraUiView $Session
    return $result.Status
}

function Initialize-SovraWpf {
    if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') { throw 'Sovra desktop UI requires an STA thread.' }
    Add-Type -AssemblyName PresentationFramework, PresentationCore, WindowsBase
}

function Update-SovraDesktopControls($Session) {
    $Session.Panel.Children.Clear()
    $Session.Buttons = @{}
    $Session.Inputs = @{}
    $Session.Status.Text = $Session.Notice
    $Session.Window.Title = $Session.Nodes[0].Text
    foreach ($node in $Session.Nodes) {
        if ($node.Kind -cin @('heading', 'text')) {
            $text = [Windows.Controls.TextBlock]::new()
            $text.Text = $node.Text
            $text.TextWrapping = 'Wrap'
            $text.Margin = '0,0,0,12'
            if ($node.Kind -ceq 'heading') { $text.FontSize = 28; $text.FontWeight = 'Bold' }
            [void]$Session.Panel.Children.Add($text)
            continue
        }
        $row = [Windows.Controls.StackPanel]::new()
        $row.Margin = '0,0,0,12'
        $textBox = $null
        if ($node.Kind -ceq 'input') {
            $label = [Windows.Controls.Label]::new()
            $labelText = [Windows.Controls.TextBlock]::new()
            $labelText.Text = $node.Label
            $label.Content = $labelText
            $textBox = [Windows.Controls.TextBox]::new()
            $textBox.MinHeight = 32
            $textBox.FontSize = 16
            $textBox.Padding = '8,4'
            $textBox.MaxLength = 65536
            $label.Target = $textBox
            [Windows.Automation.AutomationProperties]::SetName($textBox, $node.Label)
            $Session.Inputs[$node.Action] = $textBox
            [void]$row.Children.Add($label)
            [void]$row.Children.Add($textBox)
        }
        $button = [Windows.Controls.Button]::new()
        $buttonLabel = $(if ($node.Kind -ceq 'input') { $node.Submit } else { $node.Label })
        $buttonText = [Windows.Controls.TextBlock]::new()
        $buttonText.Text = $buttonLabel
        $button.Content = $buttonText
        $button.HorizontalAlignment = 'Left'
        $button.Margin = '0,6,0,0'
        $button.Padding = '16,7'
        $button.Background = '#075b72'
        $button.Foreground = 'White'
        [Windows.Automation.AutomationProperties]::SetAutomationId($button, $node.Action)
        [Windows.Automation.AutomationProperties]::SetName($button, $buttonLabel)
        $action = $node.Action
        $hasInput = $node.Kind -ceq 'input'
        $sessionForEvent = $Session
        $button.Add_Click({
            if ($sessionForEvent.Busy) { return }
            $sessionForEvent.Busy = $true
            $sessionForEvent.Window.IsEnabled = $false
            try {
                $value = $(if ($hasInput) { $textBox.Text } else { '' })
                $null = Invoke-SovraUiEvent $sessionForEvent $action $value -HasValue:$hasInput
                Update-SovraDesktopControls $sessionForEvent
            } catch { $sessionForEvent.Status.Text = $_.Exception.Message }
            finally {
                $sessionForEvent.Busy = $false
                $sessionForEvent.Window.IsEnabled = $true
            }
        }.GetNewClosure())
        $Session.Buttons[$node.Action] = $button
        [void]$row.Children.Add($button)
        [void]$Session.Panel.Children.Add($row)
    }
}

function New-SovraDesktopWindow($Session) {
    Initialize-SovraWpf
    Update-SovraUiView $Session
    $window = [Windows.Window]::new()
    $window.Width = 780
    $window.Height = 640
    $window.MinWidth = 420
    $window.MinHeight = 320
    $window.WindowStartupLocation = 'CenterScreen'
    $window.Background = '#f2f5f6'
    $root = [Windows.Controls.DockPanel]::new()
    $root.Margin = '24'
    $status = [Windows.Controls.TextBlock]::new()
    $status.TextWrapping = 'Wrap'
    $status.Margin = '0,0,0,16'
    [Windows.Automation.AutomationProperties]::SetName($status, 'Application status')
    [Windows.Controls.DockPanel]::SetDock($status, 'Top')
    [void]$root.Children.Add($status)
    $scroll = [Windows.Controls.ScrollViewer]::new()
    $scroll.VerticalScrollBarVisibility = 'Auto'
    $panel = [Windows.Controls.StackPanel]::new()
    $scroll.Content = $panel
    [void]$root.Children.Add($scroll)
    $window.Content = $root
    $Session.Window = $window
    $Session.Status = $status
    $Session.Panel = $panel
    Update-SovraDesktopControls $Session
    return $window
}
