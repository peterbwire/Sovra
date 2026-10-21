# Private Windows renderer for the existing Sovra view/event prototype.
# Dot-source for conformance tests; no windows are created on import.
Set-StrictMode -Version Latest
$script:SovraDesktopRenderer = [IO.Path]::GetFullPath($PSCommandPath)

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
    $grouped = $lines[0] -ceq 'SVR-UI-2'
    if (-not $grouped -and $lines[0] -cne 'SVR-UI-1') { throw 'Unsupported UI view version.' }
    $recordLimit = $(if ($grouped) { 768 } else { 256 })
    if ($lines.Count -lt 2 -or $lines.Count -gt $recordLimit + 1) { throw 'UI view exceeds its node limit.' }
    $nodes = @()
    $groups = @{}
    $actions = @{}
    $stack = [Collections.Generic.List[string]]::new()
    $leafCount = 0
    foreach ($line in $lines[1..($lines.Count - 1)]) {
        $split = $line.IndexOf('|')
        if ($split -lt 0) { throw 'Invalid UI node.' }
        $kind = $line.Substring(0, $split)
        $value = $line.Substring($split + 1)
        $node = [pscustomobject]@{
            Kind = $kind; Text = ''; Action = ''; Label = ''; Submit = ''
            Key = ''; Orientation = ''; Presentation = ''
        }
        if ($grouped -and $kind -ceq 'group') {
            $parts = $value.Split('|')
            if ($parts.Count -ne 3 -or $parts[0] -cnotmatch '^[a-z][a-z0-9-]{0,63}$' -or
                $parts[1] -cnotin @('vertical', 'horizontal') -or $parts[2] -cnotin @('card', 'plain')) {
                throw 'Invalid UI group.'
            }
            if ($groups.ContainsKey($parts[0]) -or $groups.Count -ge 256 -or $stack.Count -ge 8) { throw 'UI group identity/depth limit exceeded.' }
            $node.Key = $parts[0]; $node.Orientation = $parts[1]; $node.Presentation = $parts[2]
            $groups[$node.Key] = $true
            $stack.Add($node.Key)
            $nodes += $node
            continue
        }
        if ($grouped -and $kind -ceq 'end') {
            if ($stack.Count -eq 0 -or $stack[$stack.Count - 1] -cne $value) { throw 'Unbalanced UI group.' }
            $stack.RemoveAt($stack.Count - 1)
            $node.Key = $value
            $nodes += $node
            continue
        }
        $leafCount++
        if ($leafCount -gt 256) { throw 'UI view exceeds 256 leaf nodes.' }
        if ($kind -ceq 'heading' -or $kind -ceq 'text') {
            if (-not $value -or $value -match '[\x00-\x1f\x7f-\x9f]') { throw 'Invalid UI text.' }
            $node.Text = $value
        } elseif ($kind -ceq 'button' -or $kind -ceq 'input') {
            $first = $value.IndexOf('|')
            if ($first -lt 0) { throw 'Invalid UI action.' }
            $node.Action = $value.Substring(0, $first)
            if ($node.Action -cnotmatch '^[a-z][a-z0-9-]{0,63}$') { throw 'Invalid UI action key.' }
            if ($actions.ContainsKey($node.Action)) { throw 'UI action identifiers must be unique.' }
            $actions[$node.Action] = $true
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
    if ($stack.Count -ne 0) { throw 'Unclosed UI group.' }
    return $nodes
}

function New-SovraUiSession {
    param([string]$Program, [string]$State, [string]$Source = '',
        [ValidateSet('ui-view', 'ui-layout-view')][string]$ViewCommand = 'ui-view')
    if (-not (Test-Path -LiteralPath $Program -PathType Leaf)) { throw 'Sovra program was not found.' }
    if ($Source -and -not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw 'Sovra source was not found.' }
    return [pscustomobject]@{
        Program = [IO.Path]::GetFullPath($Program); State = [IO.Path]::GetFullPath($State)
        Source = $(if ($Source) { [IO.Path]::GetFullPath($Source) } else { '' })
        Nodes = @(); Notice = ''; Busy = $false; Window = $null; Panel = $null
        Status = $null; Buttons = @{}; Inputs = @{}; Rows = @{}; Scroll = $null
        TimeoutMilliseconds = 30000
        RendererPath = $script:SovraDesktopRenderer; Operation = $null; OperationTimer = $null
        CloseRequested = $false; Closed = $false; LastOperationStatus = $null
        ViewCommand = $ViewCommand; Groups = @{}
    }
}

function Update-SovraUiView($Session) {
    $view = Invoke-SovraUiProgram $Session @($Session.ViewCommand, $Session.State)
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

function Set-SovraDesktopStatus($Session, [AllowEmptyString()][string]$Text) {
    $changed = $Session.Status.Text -cne $Text
    $Session.Status.Text = $Text
    $name = $(if ($Text) { 'Application status: ' + $Text } else { 'Application status' })
    [Windows.Automation.AutomationProperties]::SetName($Session.Status, $name)
    # Older WPF releases still expose the updated accessible name. Use live
    # announcements when the platform provides them, without raising its minimum.
    $liveMethod = [Windows.Automation.AutomationProperties].GetMethod('SetLiveSetting')
    if ($liveMethod) {
        [Windows.Automation.AutomationProperties]::SetLiveSetting($Session.Status, [Windows.Automation.AutomationLiveSetting]::Polite)
        if ($changed) {
            $peer = [Windows.Automation.Peers.UIElementAutomationPeer]::CreatePeerForElement($Session.Status)
            if ($peer) { $peer.RaiseAutomationEvent([Windows.Automation.Peers.AutomationEvents]::LiveRegionChanged) }
        }
    }
}

function Get-SovraDesktopActionNames($Nodes) {
    $names = @{}
    $frames = [Collections.Generic.List[object]]::new()
    $frames.Add([pscustomobject]@{ Text = '' })
    foreach ($node in $Nodes) {
        if ($node.Kind -ceq 'group') { $frames.Add([pscustomobject]@{ Text = '' }); continue }
        if ($node.Kind -ceq 'end') { $frames.RemoveAt($frames.Count - 1); continue }
        if ($node.Kind -cin @('heading', 'text')) { $frames[$frames.Count - 1].Text = $node.Text; continue }
        if ($node.Kind -ceq 'input') { $names[$node.Action] = $node.Submit + ': ' + $node.Label; continue }
        if ($node.Kind -ceq 'button') {
            $context = ''
            for ($index = $frames.Count - 1; $index -ge 0; $index--) {
                if ($frames[$index].Text) { $context = $frames[$index].Text; break }
            }
            $names[$node.Action] = $node.Label + $(if ($context) { ': ' + $context } else { '' })
        }
    }
    return $names
}

function Start-SovraDesktopEvent {
    param($Session, [string]$Action, [string]$Value = '', [bool]$HasValue = $false)
    if ($Session.Busy -or $Session.Closed) { return $false }
    $interaction = Get-SovraDesktopInteraction $Session
    $Session.Busy = $true
    $Session.Panel.IsEnabled = $false
    Set-SovraDesktopStatus $Session 'Working...'
    $shell = $null
    $runspace = $null
    try {
        $worker = Join-Path (Split-Path -Parent $Session.RendererPath) 'worker.ps1'
        if (-not (Test-Path -LiteralPath $worker -PathType Leaf)) { throw 'The desktop worker is missing from the package.' }
        $runspace = [RunspaceFactory]::CreateRunspace()
        $runspace.ApartmentState = 'MTA'
        $runspace.ThreadOptions = 'ReuseThread'
        $runspace.Open()
        $shell = [PowerShell]::Create()
        $shell.Runspace = $runspace
        [void]$shell.AddCommand($worker).AddParameter('Program', $Session.Program).
            AddParameter('Source', $Session.Source).AddParameter('State', $Session.State).
            AddParameter('Action', $Action).AddParameter('Value', $Value).
            AddParameter('HasValue', $HasValue).AddParameter('TimeoutMilliseconds', $Session.TimeoutMilliseconds).
            AddParameter('ViewCommand', $Session.ViewCommand)
        $handle = $shell.BeginInvoke()
        $Session.Operation = [pscustomobject]@{
            Shell = $shell; Runspace = $runspace; Handle = $handle; Interaction = $interaction
            Action = $Action; Value = $Value; HasValue = $HasValue
        }
        $timer = [Windows.Threading.DispatcherTimer]::new()
        $timer.Interval = [TimeSpan]::FromMilliseconds(25)
        $sessionForTimer = $Session
        $timer.Add_Tick({ Complete-SovraDesktopEvent $sessionForTimer }.GetNewClosure())
        $Session.OperationTimer = $timer
        $timer.Start()
        return $true
    } catch {
        if ($shell) { $shell.Dispose() }
        if ($runspace) { $runspace.Dispose() }
        $Session.Operation = $null
        $Session.Busy = $false
        $Session.Panel.IsEnabled = $true
        Set-SovraDesktopStatus $Session $_.Exception.Message
        Restore-SovraDesktopInteraction $Session $interaction
        return $false
    }
}

function Complete-SovraDesktopEvent($Session) {
    $operation = $Session.Operation
    if (-not $operation -or -not $operation.Handle.IsCompleted) { return }
    $Session.OperationTimer.Stop()
    $Session.OperationTimer = $null
    $Session.Operation = $null
    $statusCode = $null
    $errorText = ''
    $nodes = @()
    try {
        $results = @($operation.Shell.EndInvoke($operation.Handle))
        if ($results.Count -ne 1) {
            $detail = ($operation.Shell.Streams.Error | ForEach-Object { $_.Exception.Message }) -join '; '
            throw ('The desktop worker did not return a valid result. ' + $detail)
        }
        $statusCode = $results[0].Status
        $errorText = $results[0].Error
        $nodes = @($results[0].Nodes)
        $Session.Notice = $results[0].Notice
    } catch { $errorText = $_.Exception.Message }
    finally {
        $operation.Shell.Dispose()
        $operation.Runspace.Dispose()
        $Session.Busy = $false
        $Session.LastOperationStatus = $statusCode
        $Session.Panel.IsEnabled = $true
    }
    # Only the dispatcher touches controls. Retain any scroll movement made while
    # the worker ran, rather than jumping back to the event's original offset.
    $operation.Interaction.Offset = $Session.Scroll.VerticalOffset
    try {
        if ($operation.HasValue -and $statusCode -eq 0 -and -not $errorText -and
            $Session.Inputs.ContainsKey($operation.Action)) {
            $field = $Session.Inputs[$operation.Action]
            if ($field.Text -ceq $operation.Value) { $field.Clear() }
        }
        if ($nodes.Count -gt 0) {
            $Session.Nodes = $nodes
            Update-SovraDesktopControls $Session
        }
        if ($errorText) { Set-SovraDesktopStatus $Session $errorText }
        else { Set-SovraDesktopStatus $Session $Session.Notice }
    } catch { $errorText = $_.Exception.Message; Set-SovraDesktopStatus $Session $errorText }
    finally { Restore-SovraDesktopInteraction $Session $operation.Interaction }
    if ($Session.CloseRequested) {
        $Session.CloseRequested = $false
        # Do not hide failures or replay uncertain operations. A successful event
        # finishes once before a requested close; a failure stays visible.
        if (-not $errorText -and $statusCode -eq 0) { $Session.Window.Close() }
    }
}

function Get-SovraDesktopInteraction($Session) {
    $state = [pscustomobject]@{ Selections = @{}; FocusAction = ''; FocusInput = $false; Offset = 0.0 }
    foreach ($action in $Session.Inputs.Keys) {
        $field = $Session.Inputs[$action]
        $state.Selections[$action] = [pscustomobject]@{
            Control = $field; Start = $field.SelectionStart; Length = $field.SelectionLength
        }
        if ($field.IsKeyboardFocusWithin) { $state.FocusAction = $action; $state.FocusInput = $true }
    }
    foreach ($action in $Session.Buttons.Keys) {
        if ($Session.Buttons[$action].IsKeyboardFocusWithin) { $state.FocusAction = $action }
    }
    if ($Session.Scroll) { $state.Offset = $Session.Scroll.VerticalOffset }
    return $state
}

function Restore-SovraDesktopInteraction($Session, $State) {
    foreach ($action in $State.Selections.Keys) {
        if ($Session.Inputs.ContainsKey($action)) {
            $saved = $State.Selections[$action]
            $field = $Session.Inputs[$action]
            if ([object]::ReferenceEquals($saved.Control, $field)) {
                $start = [Math]::Min($saved.Start, $field.Text.Length)
                $field.Select($start, [Math]::Min($saved.Length, $field.Text.Length - $start))
            }
        }
    }
    if ($State.FocusAction -and $Session.Window.IsVisible -and $Session.Window.IsActive -and $Session.Window.IsEnabled) {
        $control = $null
        if ($State.FocusInput -and $Session.Inputs.ContainsKey($State.FocusAction)) {
            $control = $Session.Inputs[$State.FocusAction]
        } elseif (-not $State.FocusInput -and $Session.Buttons.ContainsKey($State.FocusAction)) {
            $control = $Session.Buttons[$State.FocusAction]
        } else {
            foreach ($node in $Session.Nodes) {
                if ($node.Kind -ceq 'input') { $control = $Session.Inputs[$node.Action]; break }
            }
        }
        if ($control) { [void]$control.Focus() }
    }
    $Session.Window.UpdateLayout()
    if ($Session.Scroll) { $Session.Scroll.ScrollToVerticalOffset($State.Offset) }
}

function Update-SovraDesktopControls($Session) {
    $interaction = Get-SovraDesktopInteraction $Session
    $desired = [Collections.Generic.List[Windows.UIElement]]::new()
    $plans = [Collections.Generic.List[object]]::new()
    $plans.Add([pscustomobject]@{ Panel = $Session.Panel; Children = $desired })
    $parentPlans = [Collections.Generic.List[object]]::new()
    $currentPlan = $plans[0]
    $groups = @{}
    $actionNames = Get-SovraDesktopActionNames $Session.Nodes
    $tabIndex = 0
    $rows = @{}
    # A repeated action cannot identify two distinct stateful controls safely.
    $actions = @{}
    foreach ($node in $Session.Nodes) {
        if ($node.Kind -cin @('input', 'button')) {
            if ($actions.ContainsKey($node.Action)) { throw 'UI action identifiers must be unique.' }
            $actions[$node.Action] = $true
        }
    }
    $Session.Buttons = @{}
    $Session.Inputs = @{}
    Set-SovraDesktopStatus $Session $Session.Notice
    $Session.Window.Title = $Session.Nodes[0].Text
    foreach ($node in $Session.Nodes) {
        if ($node.Kind -ceq 'group') {
            $entry = $null
            if ($Session.Groups.ContainsKey($node.Key) -and $Session.Groups[$node.Key].Orientation -ceq $node.Orientation) {
                $entry = $Session.Groups[$node.Key]
            } else {
                $groupPanel = $(if ($node.Orientation -ceq 'horizontal') {
                    [Windows.Controls.WrapPanel]::new()
                } else { [Windows.Controls.StackPanel]::new() })
                $border = [Windows.Controls.Border]::new()
                $border.Child = $groupPanel
                [Windows.Input.KeyboardNavigation]::SetTabNavigation($groupPanel, [Windows.Input.KeyboardNavigationMode]::Continue)
                $entry = [pscustomobject]@{ Element = $border; Panel = $groupPanel; Orientation = $node.Orientation }
            }
            $entry.Element.Margin = '0,0,0,12'
            $entry.Element.Padding = $(if ($node.Presentation -ceq 'card') { '16' } else { '0' })
            $entry.Element.BorderThickness = $(if ($node.Presentation -ceq 'card') { '1' } else { '0' })
            $entry.Element.BorderBrush = '#dbe3e8'
            $entry.Element.CornerRadius = $(if ($node.Presentation -ceq 'card') { '10' } else { '0' })
            $entry.Element.Background = $(if ($node.Presentation -ceq 'card') { 'White' } else { 'Transparent' })
            [Windows.Automation.AutomationProperties]::SetAutomationId($entry.Element, 'group-' + $node.Key)
            $groups[$node.Key] = $entry
            $currentPlan.Children.Add($entry.Element)
            $parentPlans.Add($currentPlan)
            $currentPlan = [pscustomobject]@{ Panel = $entry.Panel; Children = [Collections.Generic.List[Windows.UIElement]]::new() }
            $plans.Add($currentPlan)
            continue
        }
        if ($node.Kind -ceq 'end') {
            $currentPlan = $parentPlans[$parentPlans.Count - 1]
            $parentPlans.RemoveAt($parentPlans.Count - 1)
            continue
        }
        if ($node.Kind -cin @('heading', 'text')) {
            $text = [Windows.Controls.TextBlock]::new()
            $text.Text = $node.Text
            $text.TextWrapping = 'Wrap'
            $text.Margin = '0,0,0,12'
            if ($currentPlan.Panel -is [Windows.Controls.WrapPanel]) {
                $text.MaxWidth = 420
                $text.Margin = '0,10,16,8'
            }
            if ($node.Kind -ceq 'heading') { $text.FontSize = 28; $text.FontWeight = 'Bold' }
            $currentPlan.Children.Add($text)
            continue
        }
        if ($Session.Rows.ContainsKey($node.Action) -and $Session.Rows[$node.Action].Kind -ceq $node.Kind) {
            $entry = $Session.Rows[$node.Action]
            $entry.Button.Content.Text = $(if ($node.Kind -ceq 'input') { $node.Submit } else { $node.Label })
            [Windows.Automation.AutomationProperties]::SetName($entry.Button, $actionNames[$node.Action])
            if ($node.Kind -ceq 'input') {
                $entry.Label.Content.Text = $node.Label
                [Windows.Automation.AutomationProperties]::SetName($entry.Input, $node.Label)
                $Session.Inputs[$node.Action] = $entry.Input
                $entry.Input.TabIndex = $tabIndex; $tabIndex++
            }
            $entry.Button.TabIndex = $tabIndex; $tabIndex++
            $rows[$node.Action] = $entry
            $Session.Buttons[$node.Action] = $entry.Button
            $entry.Row.Margin = $(if ($currentPlan.Panel -is [Windows.Controls.WrapPanel]) { '0,0,8,0' } else { '0,0,0,12' })
            $currentPlan.Children.Add($entry.Row)
            continue
        }
        $row = [Windows.Controls.StackPanel]::new()
        $row.Margin = '0,0,0,12'
        $textBox = $null
        $label = $null
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
            $label.IsTabStop = $false
            $textBox.TabIndex = $tabIndex; $tabIndex++
            [Windows.Automation.AutomationProperties]::SetAutomationId($textBox, 'input-' + $node.Action)
            [Windows.Automation.AutomationProperties]::SetName($textBox, $node.Label)
            $sessionForKey = $Session
            $actionForKey = $node.Action
            $textBox.Add_PreviewKeyDown({
                param($sender, $eventArgs)
                if ($eventArgs.Key -eq [Windows.Input.Key]::Escape -and
                    [Windows.Input.Keyboard]::Modifiers -eq [Windows.Input.ModifierKeys]::None) {
                    $eventArgs.Handled = $true
                    $sender.Clear()
                    return
                }
                if ($eventArgs.Key -eq [Windows.Input.Key]::Return -and
                    [Windows.Input.Keyboard]::Modifiers -eq [Windows.Input.ModifierKeys]::None) {
                    $eventArgs.Handled = $true
                    if (-not $eventArgs.IsRepeat) {
                        $null = Start-SovraDesktopEvent $sessionForKey $actionForKey $sender.Text $true
                    }
                }
            }.GetNewClosure())
            $Session.Inputs[$node.Action] = $textBox
            [void]$row.Children.Add($label)
            [void]$row.Children.Add($textBox)
        }
        $button = [Windows.Controls.Button]::new()
        $button.TabIndex = $tabIndex; $tabIndex++
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
        [Windows.Automation.AutomationProperties]::SetName($button, $actionNames[$node.Action])
        $action = $node.Action
        $hasInput = $node.Kind -ceq 'input'
        $sessionForEvent = $Session
        $button.Add_Click({
            $value = $(if ($hasInput) { $textBox.Text } else { '' })
            $null = Start-SovraDesktopEvent $sessionForEvent $action $value $hasInput
        }.GetNewClosure())
        $Session.Buttons[$node.Action] = $button
        [void]$row.Children.Add($button)
        $rows[$node.Action] = [pscustomobject]@{
            Kind = $node.Kind; Row = $row; Button = $button; Input = $textBox; Label = $label
        }
        if ($currentPlan.Panel -is [Windows.Controls.WrapPanel]) { $row.Margin = '0,0,8,0' }
        $currentPlan.Children.Add($row)
    }
    # Keep unchanged native controls attached. Only removed or moved nodes leave
    # the tree; drafts are never transferred to a different kind or removed field.
    foreach ($plan in $plans) {
        for ($index = $plan.Panel.Children.Count - 1; $index -ge 0; $index--) {
            if (-not $plan.Children.Contains($plan.Panel.Children[$index])) { $plan.Panel.Children.RemoveAt($index) }
        }
    }
    foreach ($plan in $plans) {
        for ($index = 0; $index -lt $plan.Children.Count; $index++) {
            $element = $plan.Children[$index]
            if ($index -lt $plan.Panel.Children.Count -and [object]::ReferenceEquals($plan.Panel.Children[$index], $element)) { continue }
            if ($element.Parent -is [Windows.Controls.Panel]) { $element.Parent.Children.Remove($element) }
            $plan.Panel.Children.Insert($index, $element)
        }
    }
    $Session.Rows = $rows
    $Session.Groups = $groups
    Restore-SovraDesktopInteraction $Session $interaction
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
    [Windows.Input.KeyboardNavigation]::SetTabNavigation($window, [Windows.Input.KeyboardNavigationMode]::Cycle)
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
    $scroll.IsTabStop = $false
    $panel = [Windows.Controls.StackPanel]::new()
    $scroll.Content = $panel
    [void]$root.Children.Add($scroll)
    $window.Content = $root
    $Session.Window = $window
    $Session.Status = $status
    $Session.Panel = $panel
    $Session.Scroll = $scroll
    $sessionForClose = $Session
    $window.Add_Closing({
        param($sender, $eventArgs)
        if ($sessionForClose.Busy) {
            $eventArgs.Cancel = $true
            $sessionForClose.CloseRequested = $true
            Set-SovraDesktopStatus $sessionForClose 'Finishing the current operation before closing...'
        }
    }.GetNewClosure())
    $window.Add_Closed({ $sessionForClose.Closed = $true }.GetNewClosure())
    Update-SovraDesktopControls $Session
    return $window
}
