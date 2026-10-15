param(
    [string]$Compiler = 'target\release\svr.exe',
    [string]$TaskProgram = '',
    [string]$ComponentProgram = '',
    [string]$Renderer = ''
)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Renderer) { $Renderer = Join-Path $repository 'toolchain\ui\windows\renderer.ps1' }
. $Renderer
function Assert-Ui($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Expect-UiError([scriptblock]$Operation) {
    $failed = $false
    try { & $Operation | Out-Null } catch { $failed = $true }
    Assert-Ui $failed 'Invalid UI input was accepted.'
}
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-desktop-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch | Out-Null
$taskWindow = $null
$componentWindow = $null
$failureWindow = $null
try {
    foreach ($bad in @('SVR-UI-1', "SVR-UI-2`nheading|Wrong version`n", "SVR-UI-1`ntext|Missing heading`n",
            "SVR-UI-1`nheading|Hello`nbutton|Bad Key|No`n", "SVR-UI-1`nheading|Bad`ttext`n",
            "SVR-UI-1`nheading|Hello`ninput|save|Label`n", "SVR-UI-1`nheading|Hello`nscript|bad`n",
            ("SVR-UI-1`nheading|Hello`n" + ("text|more`n" * 256)),
            ("SVR-UI-1`nheading|" + ('x' * 65536) + "`n"))) {
        Expect-UiError { ConvertFrom-SovraUiView $bad }
    }
    $valid = @(ConvertFrom-SovraUiView "SVR-UI-1`nheading|Hello <literal>`ninput|save|Text|Save`nbutton|reset|Reset`n")
    Assert-Ui ($valid.Count -eq 3) 'Valid UI view was rejected.'
    $compilerPath = [IO.Path]::GetFullPath($Compiler)
    $state = Join-Path $scratch 'task state !.txt'
    if ($TaskProgram) { $session = New-SovraUiSession -Program $TaskProgram -State $state }
    else { $session = New-SovraUiSession -Program $compilerPath -State $state -Source (Join-Path $repository 'dogfood\01-task-manager\main.svr') }
    Update-SovraUiView $session
    Assert-Ui ($session.Nodes[0].Text -ceq 'Sovra Task Manager') 'Task view did not come from Sovra.'
    $before = $session.Nodes.Count
    Expect-UiError { Invoke-SovraUiEvent $session 'not-offered' }
    Assert-Ui (-not (Test-Path -LiteralPath $state)) 'Rejected action modified state.'
    Expect-UiError { Invoke-SovraUiEvent $session 'add' }
    $title = 'Literal <UI> "quotes" & $(not code) caf' + [char]0x00e9 + ' trailing\'
    Assert-Ui ((Invoke-SovraUiEvent $session 'add' $title -HasValue) -eq 0) 'Sovra add event failed.'
    Assert-Ui (@($session.Nodes | Where-Object { $_.Text.Contains($title) }).Count -eq 1) 'Process argv did not preserve literal title.'
    $bytesBefore = [IO.File]::ReadAllText($state)
    Expect-UiError { Invoke-SovraUiEvent $session 'complete-1' 'unexpected' -HasValue }
    Assert-Ui ([IO.File]::ReadAllText($state) -ceq $bytesBefore) 'Invalid event changed state.'
    Assert-Ui ((Invoke-SovraUiEvent $session 'complete-1') -eq 0) 'Sovra complete event failed.'
    Expect-UiError { Invoke-SovraUiEvent $session 'complete-1' }
    Assert-Ui ((Invoke-SovraUiEvent $session 'delete-1') -eq 0) 'Sovra delete event failed.'
    Assert-Ui ($session.Nodes.Count -eq $before) 'View failed to return to empty state.'
    $taskWindow = New-SovraDesktopWindow $session
    Assert-Ui ($session.Inputs['add'] -is [Windows.Controls.TextBox]) 'Renderer did not build a native text input.'
    Assert-Ui ($session.Buttons['add'] -is [Windows.Controls.Button]) 'Renderer did not build a native button.'
    Assert-Ui ([Windows.Automation.AutomationProperties]::GetName($session.Inputs['add']) -ceq 'Task title') 'Input has no accessible name.'
    $session.Inputs['add'].Text = 'Raised from native control'
    $session.Buttons['add'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Assert-Ui ($session.Notice -match '^added ') ('Native control event did not execute Sovra: ' + $session.Status.Text)
    Assert-Ui ($session.Buttons.ContainsKey('complete-2')) 'Native control tree did not refresh.'
    $fixtureState = Join-Path $scratch 'unrelated component.txt'
    if ($ComponentProgram) { $fixture = New-SovraUiSession -Program $ComponentProgram -State $fixtureState }
    else { $fixture = New-SovraUiSession -Program $compilerPath -State $fixtureState -Source (Join-Path $repository 'examples\ui-desktop\component.svr') }
    $componentWindow = New-SovraDesktopWindow $fixture
    $fixture.Inputs['save-message'].Text = $title
    $fixture.Buttons['save-message'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Assert-Ui (@($fixture.Nodes | Where-Object { $_.Text.Contains($title) }).Count -eq 1) ('Unrelated native component failed: ' + $fixture.Status.Text)
    $fixture.Buttons['reset-message'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Assert-Ui (-not ($fixture.Nodes | Where-Object { $_.Text.Contains($title) })) 'Unrelated reset failed.'
    $utf8 = [Text.UTF8Encoding]::new($false)
    $failureSource = Join-Path $scratch 'silent-error.svr'
    [IO.File]::WriteAllText($failureSource,
        'fn main() { if std::arg(0) == "ui-view" { print("SVR-UI-1"); print("heading|Failure fixture"); print("button|reject|Reject_literal"); } else { std::set_exit_code(7); } }', $utf8)
    $failure = New-SovraUiSession -Program $compilerPath -Source $failureSource -State $state
    $failureWindow = New-SovraDesktopWindow $failure
    Assert-Ui ($failure.Buttons['reject'].Content.Text -ceq 'Reject_literal') 'A label was interpreted as access-key markup.'
    $failure.Buttons['reject'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Assert-Ui ($failure.Status.Text -match 'Operation failed \(exit 7\)') 'Silent nonzero event was not shown as an error.'
    [IO.File]::WriteAllText($state, 'corrupt task data', $utf8)
    Expect-UiError { Update-SovraUiView $session }
    Assert-Ui ([IO.File]::ReadAllText($state) -ceq 'corrupt task data') 'Bad state was overwritten.'
    $missing = New-SovraUiSession -Program $compilerPath -State $state
    $missing.Program = Join-Path $scratch 'missing.exe'
    Expect-UiError { Invoke-SovraUiProgram $missing @('ui-view', $state) }
    $noisySource = Join-Path $scratch 'noisy.svr'
    [IO.File]::WriteAllText($noisySource,
        'fn main() { let mut count = 0; while count < 10000 { print("more output than the renderer permits"); count = count + 1; } }', $utf8)
    $noisy = New-SovraUiSession -Program $compilerPath -Source $noisySource -State $state
    Expect-UiError { Invoke-SovraUiProgram $noisy @() }
    $loopSource = Join-Path $scratch 'loop.svr'
    [IO.File]::WriteAllText($loopSource, 'fn main() { while true {} }', $utf8)
    $loop = New-SovraUiSession -Program $compilerPath -Source $loopSource -State $state
    $loop.TimeoutMilliseconds = 250
    Expect-UiError { Invoke-SovraUiProgram $loop @() }
    Write-Output 'PASS Sovra desktop UI: strict views, native controls/events, literal argv, persistence, stale actions, unrelated reuse and errors.'
} finally {
    if ($taskWindow) { $taskWindow.Close() }
    if ($componentWindow) { $componentWindow.Close() }
    if ($failureWindow) { $failureWindow.Close() }
    $resolved = [IO.Path]::GetFullPath($scratch)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-desktop-test-'
    if (-not $resolved.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected test cleanup path.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
