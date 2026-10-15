param(
    [Parameter(Mandatory=$true)][string]$Package,
    [Parameter(Mandatory=$true)][string]$Report
)
$ErrorActionPreference = 'Stop'
$archive = [IO.Path]::GetFullPath($Package)
$reportPath = [IO.Path]::GetFullPath($Report)
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-downloaded-native-' + [guid]::NewGuid().ToString('N'))
$previousPath = $env:Path
$evidence = [ordered]@{
    schema = 1
    success = $false
    archive = $archive
    archive_sha256 = $null
    binary_sha256 = $null
    architecture = $null
    imports = @()
    build = $null
    windows = [Environment]::OSVersion.VersionString
    process_bits = [IntPtr]::Size * 8
    developer_tools_absent_from_path = $false
    clean_machine_qualified = $false
    error = $null
}
try {
    $env:Path = "$env:SystemRoot\System32;$env:SystemRoot"
    foreach ($tool in @('node.exe','cargo.exe','rustc.exe','svr.exe')) {
        if (Get-Command $tool -ErrorAction SilentlyContinue) {
            throw "Developer tool unexpectedly resolves on restricted PATH: $tool"
        }
    }
    $evidence.developer_tools_absent_from_path = $true
    $evidence.archive_sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    & (Join-Path $PSScriptRoot 'test-native-task-manager-artifact.ps1') `
        -Package $archive -ExtractTo $scratch | Out-Null
    $folders = @(Get-ChildItem -LiteralPath $scratch -Directory)
    $root = $folders[0].FullName
    $evidence.binary_sha256 = (Get-FileHash -LiteralPath (Join-Path $root 'task-manager.exe') `
        -Algorithm SHA256).Hash.ToLowerInvariant()
    $audit = Get-Content -LiteralPath (Join-Path $root 'imports.json') -Raw | ConvertFrom-Json
    $evidence.architecture = $audit.architecture
    $evidence.imports = @($audit.imports)
    $evidence.build = Get-Content -LiteralPath (Join-Path $root 'build.json') -Raw | ConvertFrom-Json
    $evidence.success = $true
    Write-Output 'Downloaded native artifact passed without developer tools on PATH.'
} catch {
    $evidence.error = $_.Exception.Message
    throw
} finally {
    $env:Path = $previousPath
    New-Item -ItemType Directory -Path (Split-Path -Parent $reportPath) -Force | Out-Null
    [IO.File]::WriteAllText($reportPath, ($evidence | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-downloaded-native-'
    $resolved = [IO.Path]::GetFullPath($scratch)
    if (-not $resolved.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Unsafe downloaded native extraction cleanup path.'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
