param([string]$Compiler = 'target\release\svr.exe', [string]$Renderer = '', [string]$TaskProgram = '', [switch]$ShowWindow)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Renderer) { $Renderer = Join-Path $repository 'toolchain\ui\windows\renderer.ps1' }
. $Renderer
. (Join-Path $PSScriptRoot 'desktop-test-helpers.ps1')
function Assert-Accessible($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-accessible-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch | Out-Null
$window = $null
$session = $null
$timer = $null
try {
    if ($TaskProgram) { $session = New-SovraUiSession -Program $TaskProgram -State (Join-Path $scratch 'tasks.txt') -ViewCommand ui-layout-view }
    else { $session = New-SovraUiSession -Program ([IO.Path]::GetFullPath($Compiler)) -Source (Join-Path $repository 'dogfood\01-task-manager\main.svr') -State (Join-Path $scratch 'tasks.txt') -ViewCommand ui-layout-view }
    $null = Invoke-SovraUiEvent $session 'add' 'Keyboard task one' -HasValue
    $null = Invoke-SovraUiEvent $session 'add' 'Keyboard task two' -HasValue
    $window = New-SovraDesktopWindow $session
    $firstName = [Windows.Automation.AutomationProperties]::GetName($session.Buttons['complete-1'])
    $secondName = [Windows.Automation.AutomationProperties]::GetName($session.Buttons['complete-2'])
    Assert-Accessible ($firstName.Contains('Keyboard task one') -and $secondName.Contains('Keyboard task two') -and $firstName -cne $secondName) 'Repeated buttons have indistinguishable accessible names.'
    Assert-Accessible (-not $session.Scroll.IsTabStop) 'The scroll wrapper interrupts Tab navigation.'
    Assert-Accessible ($session.Inputs['add'].TabIndex -lt $session.Buttons['add'].TabIndex -and $session.Buttons['add'].TabIndex -lt $session.Buttons['complete-1'].TabIndex) 'Tab order does not follow the Sovra view.'
    if ([Windows.Automation.AutomationProperties].GetMethod('GetLiveSetting')) {
        Assert-Accessible ([Windows.Automation.AutomationProperties]::GetLiveSetting($session.Status).ToString() -eq 'Polite') 'Status has no live-region metadata.'
    }
    if (-not $ShowWindow) { Write-Output 'PASS desktop accessibility: contextual names and declared Tab order.'; return }
    $result = [pscustomobject]@{ Step = 0; Error = ''; Passed = $false }
    $timer = [Windows.Threading.DispatcherTimer]::new()
    $timer.Interval = [TimeSpan]::FromMilliseconds(400)
    $timer.Add_Tick({
        try {
            if ($session.Busy) { return }
            switch ($result.Step) {
                0 {
                    [void]$window.Activate()
                    $field = $session.Inputs['add']
                    [void]$field.Focus()
                    $field.Text = 'Submitted from Enter'
                    $cancel = [Windows.Input.KeyEventArgs]::new([Windows.Input.Keyboard]::PrimaryDevice,
                        [Windows.PresentationSource]::FromVisual($field), [Environment]::TickCount, [Windows.Input.Key]::Escape)
                    $cancel.RoutedEvent = [Windows.Input.Keyboard]::PreviewKeyDownEvent
                    $field.RaiseEvent($cancel)
                    Assert-Accessible ($cancel.Handled -and $field.Text -ceq '' -and -not $session.Busy) 'Escape did not cancel the local draft without an application event.'
                    $field.Text = 'Submitted from Enter'
                    $letter = [Windows.Input.KeyEventArgs]::new([Windows.Input.Keyboard]::PrimaryDevice,
                        [Windows.PresentationSource]::FromVisual($field), [Environment]::TickCount, [Windows.Input.Key]::A)
                    $letter.RoutedEvent = [Windows.Input.Keyboard]::PreviewKeyDownEvent
                    $field.RaiseEvent($letter)
                    Assert-Accessible (-not $letter.Handled -and -not $session.Busy) 'Ordinary editing keys triggered an operation.'
                    $key = [Windows.Input.KeyEventArgs]::new([Windows.Input.Keyboard]::PrimaryDevice,
                        [Windows.PresentationSource]::FromVisual($field), [Environment]::TickCount, [Windows.Input.Key]::Return)
                    $key.RoutedEvent = [Windows.Input.Keyboard]::PreviewKeyDownEvent
                    $field.RaiseEvent($key)
                    Assert-Accessible ($key.Handled -and $session.Busy) 'Enter did not submit the focused input.'
                    $operation = $session.Operation
                    $again = [Windows.Input.KeyEventArgs]::new([Windows.Input.Keyboard]::PrimaryDevice,
                        [Windows.PresentationSource]::FromVisual($field), [Environment]::TickCount, [Windows.Input.Key]::Return)
                    $again.RoutedEvent = [Windows.Input.Keyboard]::PreviewKeyDownEvent
                    $field.RaiseEvent($again)
                    Assert-Accessible ([object]::ReferenceEquals($operation, $session.Operation)) 'Repeated Enter replaced the in-flight operation.'
                    Assert-Accessible ([Windows.Automation.AutomationProperties]::GetName($session.Status).Contains('Working')) 'Busy status is not exposed to automation.'
                }
                1 {
                    Assert-Accessible ($session.Buttons.ContainsKey('complete-3')) ('Keyboard submission failed: ' + $session.Status.Text)
                    Assert-Accessible ($session.Inputs['add'].IsKeyboardFocused -and $session.Inputs['add'].Text -ceq '') 'Keyboard submission lost input focus.'
                    Assert-Accessible ([Windows.Automation.AutomationProperties]::GetName($session.Status).Contains('added 3')) 'Completion status is not accessible.'
                    $session.Inputs['add'].MoveFocus([Windows.Input.TraversalRequest]::new([Windows.Input.FocusNavigationDirection]::Next)) | Out-Null
                    Assert-Accessible ($session.Buttons['add'].IsKeyboardFocused) 'Tab did not move from input to its submit button.'
                    $session.Buttons['add'].MoveFocus([Windows.Input.TraversalRequest]::new([Windows.Input.FocusNavigationDirection]::Next)) | Out-Null
                    Assert-Accessible ($session.Buttons['complete-1'].IsKeyboardFocused) 'Tab order changed across groups.'
                    $session.Buttons['complete-1'].MoveFocus([Windows.Input.TraversalRequest]::new([Windows.Input.FocusNavigationDirection]::Previous)) | Out-Null
                    Assert-Accessible ($session.Buttons['add'].IsKeyboardFocused) 'Reverse Tab order is inconsistent.'
                    $peer = [Windows.Automation.Peers.UIElementAutomationPeer]::CreatePeerForElement($session.Buttons['complete-1'])
                    $invoke = $peer.GetPattern([Windows.Automation.Peers.PatternInterface]::Invoke)
                    Assert-Accessible ($null -ne $invoke) 'Native button has no automation Invoke pattern.'
                    $invoke.Invoke()
                }
                2 {
                    Assert-Accessible (-not $session.Buttons.ContainsKey('complete-1')) 'Automation activation did not execute Sovra.'
                    $result.Passed = $true
                    $timer.Stop()
                    $window.Close()
                }
            }
            $result.Step++
        } catch { $result.Error = $_.Exception.Message; $timer.Stop(); $window.Close() }
    }.GetNewClosure())
    $timer.Start()
    [void]$window.ShowDialog()
    Assert-Accessible $result.Passed ('Keyboard/automation regression failed: ' + $result.Error)
    Write-Output 'PASS visible desktop accessibility: Enter, Tab/reverse Tab, focus, contextual names, status and automation activation.'
} finally {
    if ($timer) { $timer.Stop() }
    if ($session -and $session.Busy) { Wait-SovraDesktopIdle $session }
    if ($window) { $window.Close() }
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-accessible-'
    if (-not [IO.Path]::GetFullPath($scratch).StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected accessibility test path.' }
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}
