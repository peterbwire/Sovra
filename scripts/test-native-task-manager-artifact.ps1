param(
    [Parameter(Mandatory=$true)][string]$Package,
    [Parameter(Mandatory=$true)][string]$ExtractTo
)
$ErrorActionPreference = 'Stop'
$archive = [IO.Path]::GetFullPath($Package)
$destination = [IO.Path]::GetFullPath($ExtractTo)
if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) { throw "Archive missing: $archive" }
if (Test-Path -LiteralPath $destination) { throw "Extraction path already exists: $destination" }
$line = (Get-Content -LiteralPath "$archive.sha256" -Raw).Trim()
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($line -cne "$hash  $([IO.Path]::GetFileName($archive))") {
    throw 'Native Task Manager ZIP checksum mismatch.'
}
Expand-Archive -LiteralPath $archive -DestinationPath $destination
$folders = @(Get-ChildItem -LiteralPath $destination -Directory)
if ($folders.Count -ne 1) { throw 'Native Task Manager ZIP has an unexpected root layout.' }
$root = $folders[0].FullName
$binary = Join-Path $root 'task-manager.exe'
$build = Get-Content -LiteralPath (Join-Path $root 'build.json') -Raw | ConvertFrom-Json
if ($build.schema -ne 1 -or $build.method -cne 'svr build --emit exe --output' -or
    $build.compiler_sha256 -cnotmatch '^[0-9a-f]{64}$' -or
    $build.source_sha256 -cnotmatch '^[0-9a-f]{64}$' -or
    $build.compiler_version -notmatch '^svr ' -or $build.architecture -ne 'x86_64') {
    throw 'Native Task Manager build provenance is invalid.'
}
$recorded = Get-Content -LiteralPath (Join-Path $root 'imports.json') -Raw | ConvertFrom-Json
$actual = & (Join-Path $PSScriptRoot 'audit-windows-imports.ps1') $binary | ConvertFrom-Json
if ($recorded.architecture -ne $actual.architecture -or
    (@($recorded.imports) -join ',') -ne (@($actual.imports) -join ',')) {
    throw 'Native Task Manager import record differs from the packaged executable.'
}
& (Join-Path $root 'verify.ps1') | Out-Null
Write-Output "Verified extracted native Task Manager: $binary"
