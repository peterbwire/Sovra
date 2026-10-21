param([string]$Compiler = 'target\release\svr.exe', [string]$TaskProgram = '', [string]$Renderer = '')
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Renderer) { $Renderer = Join-Path $repository 'toolchain\ui\windows\renderer.ps1' }
. $Renderer
. (Join-Path $PSScriptRoot 'desktop-test-helpers.ps1')
function Assert-Layout($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Expect-LayoutError([scriptblock]$Operation) {
    $failed = $false
    try { & $Operation | Out-Null } catch { $failed = $true }
    Assert-Layout $failed 'An invalid grouped view was accepted.'
}
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-layout-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $scratch | Out-Null
$windows = @()
$sessions = @()
try {
    foreach ($bad in @(
        "SVR-UI-2`nheading|Bad`ngroup|one|vertical|card`n",
        "SVR-UI-2`nheading|Bad`nend|one`n",
        "SVR-UI-2`nheading|Bad`ngroup|one|vertical|card`nend|other`n",
        "SVR-UI-2`nheading|Bad`ngroup|one|diagonal|card`nend|one`n",
        "SVR-UI-2`nheading|Bad`ngroup|one|vertical|html`nend|one`n",
        "SVR-UI-2`nheading|Bad`ngroup|one|vertical|card|extra`nend|one`n",
        "SVR-UI-2`nheading|Bad`ngroup|one|vertical|card`nend|one`ngroup|one|vertical|card`nend|one`n",
        "SVR-UI-2`nheading|Bad`ninput|save|Input|Save`nbutton|save|Duplicate`n",
        "SVR-UI-1`nheading|Old version`ngroup|one|vertical|card`nend|one`n")) {
        Expect-LayoutError { ConvertFrom-SovraUiView $bad }
    }
    $deep = "SVR-UI-2`nheading|Deep`n"
    for ($index = 0; $index -lt 9; $index++) { $deep += "group|level-$index|vertical|plain`n" }
    for ($index = 8; $index -ge 0; $index--) { $deep += "end|level-$index`n" }
    Expect-LayoutError { ConvertFrom-SovraUiView $deep }
    $capacity = "SVR-UI-2`nheading|Capacity`n"
    for ($index = 0; $index -lt 255; $index++) { $capacity += "group|row-$index|horizontal|plain`ntext|Row`nend|row-$index`n" }
    Assert-Layout (@(ConvertFrom-SovraUiView $capacity).Count -eq 766) 'Group markers reduced the primitive-node capacity.'
    Expect-LayoutError { ConvertFrom-SovraUiView ($capacity + "text|Too many leaves`n") }
    $tooManyGroups = "SVR-UI-2`nheading|Groups`n"
    for ($index = 0; $index -lt 257; $index++) { $tooManyGroups += "group|row-$index|vertical|plain`nend|row-$index`n" }
    Expect-LayoutError { ConvertFrom-SovraUiView $tooManyGroups }
    $compilerPath = [IO.Path]::GetFullPath($Compiler)
    $state = Join-Path $scratch 'tasks.txt'
    if ($TaskProgram) { $task = New-SovraUiSession -Program $TaskProgram -State $state -ViewCommand ui-layout-view }
    else { $task = New-SovraUiSession -Program $compilerPath -Source (Join-Path $repository 'dogfood\01-task-manager\main.svr') -State $state -ViewCommand ui-layout-view }
    $sessions += $task
    $windows += New-SovraDesktopWindow $task
    Assert-Layout ($task.Groups['add-task'].Element -is [Windows.Controls.Border]) 'Sovra input card was not rendered.'
    $inputControl = $task.Inputs['add']
    $inputControl.Text = 'Grouped task'
    $task.Buttons['add'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ($task.Groups.ContainsKey('task-1')) ('Task group did not refresh: ' + $task.Status.Text)
    Assert-Layout ($task.Groups['task-1'].Panel -is [Windows.Controls.WrapPanel]) 'Task actions are not in a generic wrapping row.'
    Assert-Layout ([object]::ReferenceEquals($task.Rows['complete-1'].Row.Parent, $task.Groups['task-1'].Panel)) 'Action was placed outside its Sovra group.'
    $task.Inputs['add'].Text = 'Unsubmitted draft'
    $task.Inputs['rename-1'].Text = ''
    $task.Buttons['rename-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ($task.Status.Text.Contains('empty title')) 'Empty rename was not rejected.'
    $task.Inputs['rename-1'].Text = 'Edited | task'
    $task.Buttons['rename-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ($task.Status.Text.Contains('renamed 1') -and $task.Inputs['rename-1'].Text -ceq '') 'Rename did not save and clear its own draft.'
    Assert-Layout ([IO.File]::ReadAllText($state).Contains('1|0|Edited | task')) 'Rename changed identity or lost literal title text.'
    $task.Buttons['complete-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    $task.Inputs['rename-1'].Text = 'Completed edited task'
    $task.Buttons['rename-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ([IO.File]::ReadAllText($state).Contains('1|1|Completed edited task')) 'Rename lost completion status.'
    $savedTasks = [IO.File]::ReadAllText($state)
    $task.Buttons['filter-open'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout (-not $task.Inputs.ContainsKey('rename-1')) 'Open filter retained a completed task.'
    $task.Buttons['filter-completed'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ($task.Inputs.ContainsKey('rename-1')) 'Completed filter lost a completed task.'
    $task.Inputs['search'].Text = 'no matching title'
    $task.Buttons['search'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout (-not $task.Inputs.ContainsKey('rename-1')) 'Search did not narrow results.'
    $task.Buttons['clear-search'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ($task.Inputs.ContainsKey('rename-1')) 'Clear search did not restore filtered results.'
    $task.Buttons['filter-all'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout ([IO.File]::ReadAllText($state) -ceq $savedTasks) 'View changes mutated task data.'
    Assert-Layout ([object]::ReferenceEquals($inputControl, $task.Inputs['add']) -and $inputControl.Text -ceq 'Unsubmitted draft') 'Grouped update lost an input draft.'
    $task.Buttons['delete-1'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $task
    Assert-Layout (-not $task.Groups.ContainsKey('task-1')) 'Removed task group survived.'
    $fixture = New-SovraUiSession -Program $compilerPath -Source (Join-Path $repository 'examples\ui-desktop\component.svr') -State (Join-Path $scratch 'message.txt') -ViewCommand ui-layout-view
    $sessions += $fixture
    $windows += New-SovraDesktopWindow $fixture
    $fixture.Inputs['save-message'].Text = 'Reusable card'
    $fixture.Buttons['save-message'].RaiseEvent([Windows.RoutedEventArgs]::new([Windows.Controls.Button]::ClickEvent))
    Wait-SovraDesktopIdle $fixture
    Assert-Layout ($fixture.Groups.ContainsKey('message-card') -and @($fixture.Nodes | Where-Object { $_.Text -ceq 'Reusable card' }).Count -eq 1) 'Unrelated grouped component failed.'
    $movingSource = Join-Path $scratch 'moving.svr'
    $movingState = Join-Path $scratch 'moving.txt'
    $utf8 = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText($movingSource,
        'fn main() { let state = std::read_text(std::arg(1)); print("SVR-UI-2"); print("heading|Moving component"); if state.text == "horizontal" { print("group|left|horizontal|card"); } else { print("group|left|vertical|card"); } if state.text != "right" { print("input|value|Value|Save"); } print("end|left"); print("group|right|vertical|card"); if state.text == "right" { print("input|value|Value|Save"); } print("end|right"); }', $utf8)
    $moving = New-SovraUiSession -Program $compilerPath -Source $movingSource -State $movingState -ViewCommand ui-layout-view
    $sessions += $moving
    $windows += New-SovraDesktopWindow $moving
    $field = $moving.Inputs['value']; $field.Text = 'Draft across panels'; $field.Select(2, 4)
    foreach ($placement in @('right', 'horizontal')) {
        [IO.File]::WriteAllText($movingState, $placement, $utf8)
        Update-SovraUiView $moving
        Update-SovraDesktopControls $moving
        Assert-Layout ([object]::ReferenceEquals($field, $moving.Inputs['value']) -and $field.Text -ceq 'Draft across panels' -and $field.SelectionStart -eq 2 -and $field.SelectionLength -eq 4) 'Moving a field between groups lost its state.'
    }
    Assert-Layout ($moving.Groups['left'].Panel -is [Windows.Controls.WrapPanel]) 'Orientation change did not update the native panel.'
    $invalidSource = Join-Path $scratch 'invalid.svr'
    $marker = Join-Path $scratch 'must-not-write.txt'
    [IO.File]::WriteAllText($invalidSource,
        'fn main() { if std::arg(0) == "ui-layout-view" { print("SVR-UI-2"); print("heading|Broken"); print("group|open|vertical|card"); print("button|save|Save"); } else { let result = std::write_text(std::arg(1), "unexpected mutation"); } }', $utf8)
    $invalid = New-SovraUiSession -Program $compilerPath -Source $invalidSource -State $marker -ViewCommand ui-layout-view
    Expect-LayoutError { Invoke-SovraUiEvent $invalid 'save' }
    Assert-Layout (-not (Test-Path -LiteralPath $marker)) 'An invalid layout permitted an application mutation.'
    Write-Output 'PASS grouped desktop UI: strict bounded trees, native cards/wrapping rows, real Sovra events, unrelated reuse and cross-group drafts.'
} finally {
    foreach ($session in $sessions) { if ($session.Busy) { Wait-SovraDesktopIdle $session } }
    foreach ($window in $windows) { $window.Close() }
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-layout-'
    if (-not [IO.Path]::GetFullPath($scratch).StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected layout test path.' }
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}
