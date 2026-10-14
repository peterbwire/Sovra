$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($PSScriptRoot)
$binary = Join-Path $root 'task-manager.exe'
$checksum = Join-Path $root 'task-manager.exe.sha256'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf) -or
    -not (Test-Path -LiteralPath $checksum -PathType Leaf)) {
    throw 'Native Task Manager executable or checksum is missing.'
}
$expected = (Get-Content -LiteralPath $checksum -Raw).Trim()
$actual = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expected -cne "$actual  task-manager.exe") { throw 'Native Task Manager checksum mismatch.' }
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-native-proof-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $state = Join-Path $scratch 'tasks.txt'
    $added = & $binary add $state 'Native proof'
    if ($LASTEXITCODE -ne 0 -or $added -ne 'added 1') { throw "Native add failed: $added" }
    $listed = & $binary list $state
    if ($LASTEXITCODE -ne 0 -or ($listed -join "`n") -notmatch '1 \| open \| Native proof') {
        throw "Native list failed: $listed"
    }
    $completed = & $binary complete $state 'Native proof'
    if ($LASTEXITCODE -ne 0 -or $completed -ne 'completed 1') {
        throw "Native complete failed: $completed"
    }
    $deleted = & $binary delete $state 'Native proof'
    if ($LASTEXITCODE -ne 0 -or $deleted -ne 'deleted 1') { throw "Native delete failed: $deleted" }
    $corrupt = Join-Path $scratch 'corrupt.txt'
    [IO.File]::WriteAllText($corrupt, 'invalid state', [Text.UTF8Encoding]::new($false))
    & $binary list $corrupt 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { throw 'Native program accepted corrupt state.' }
    if ([IO.File]::ReadAllText($corrupt) -cne 'invalid state') {
        throw 'Native program modified corrupt state.'
    }
    Write-Output "Verified native Task Manager from $root"
} finally {
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    $resolved = [IO.Path]::GetFullPath($scratch)
    if (-not $resolved.StartsWith($tempRoot + '\sovra-native-proof-',
            [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove an unexpected self-test path.'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
