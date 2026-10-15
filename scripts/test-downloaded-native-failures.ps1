param([Parameter(Mandatory=$true)][string]$Package)
$ErrorActionPreference = 'Stop'
$archive = [IO.Path]::GetFullPath($Package)
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-native-negative-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $damaged = Join-Path $scratch ([IO.Path]::GetFileName($archive))
    Copy-Item -LiteralPath $archive -Destination $damaged
    Copy-Item -LiteralPath "$archive.sha256" -Destination "$damaged.sha256"
    $bytes = [IO.File]::ReadAllBytes($damaged)
    $bytes[0] = $bytes[0] -bxor 1
    [IO.File]::WriteAllBytes($damaged, $bytes)
    $report = Join-Path $scratch 'failure.json'
    $powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $powershell -NoProfile -ExecutionPolicy Bypass -File `
            (Join-Path $PSScriptRoot 'test-downloaded-native.ps1') `
            -Package $damaged -Report $report 2>&1 | Out-Null
        $status = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    if ($status -eq 0) { throw 'Damaged native ZIP was accepted.' }
    $evidence = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
    if ($evidence.success -ne $false -or $evidence.error -notmatch 'ZIP checksum mismatch' -or
        $evidence.clean_machine_qualified -ne $false -or $evidence.binary_sha256 -ne $null) {
        throw 'Damaged native ZIP did not produce an accurate failure report.'
    }
    $actual = (Get-FileHash -LiteralPath $damaged -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($evidence.archive_sha256 -cne $actual) { throw 'Failure report did not identify the damaged bytes.' }
    Write-Output 'Rejected corrupted native download and retained accurate failure evidence.'
} finally {
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-native-negative-'
    $resolved = [IO.Path]::GetFullPath($scratch)
    if (-not $resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Unsafe negative-test cleanup path.'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
