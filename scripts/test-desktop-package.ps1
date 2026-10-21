param(
    [Parameter(Mandatory=$true)][Alias('Package')][string]$PackageZIP,
    [Parameter(Mandatory=$true)][string]$ExtractTo
)
$ErrorActionPreference = 'Stop'
$archive = [IO.Path]::GetFullPath($PackageZIP)
$destination = [IO.Path]::GetFullPath($ExtractTo)
if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) { throw "Archive missing: $archive" }
if (Test-Path -LiteralPath $destination) { throw "Extraction path already exists: $destination" }
$checksum = (Get-Content -LiteralPath "$archive.sha256" -Raw).Trim()
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($checksum -cne "$hash  $([IO.Path]::GetFileName($archive))") {
    throw 'Desktop UI ZIP checksum mismatch.'
}

function Assert-SafeRelativePath([string]$Name) {
    if (-not $Name -or $Name.Contains('\') -or $Name.StartsWith('/')) {
        throw 'Desktop UI ZIP contains an unsafe relative path.'
    }
    foreach ($component in $Name.Split('/')) {
        if (-not $component -or $component -eq '.' -or $component -eq '..' -or
            $component -match '[<>:"|?*\x00-\x1f]' -or $component -match '[. ]$' -or
            $component -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)') {
            throw 'Desktop UI ZIP contains an unsafe relative path.'
        }
    }
}

# Validate every member before extraction; do not trust Expand-Archive to define
# the package boundary or interpret ambiguous Windows paths safely.
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
$memberNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$roots = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
try {
    foreach ($entry in $zip.Entries) {
        # Windows Compress-Archive writes backslash separators; validate their
        # canonical slash form with the same duplicate and traversal rules.
        $name = $entry.FullName.Replace('\', '/')
        $directory = $name.EndsWith('/')
        if ($directory) { $name = $name.Substring(0, $name.Length - 1) }
        Assert-SafeRelativePath $name
        if (-not $memberNames.Add($name)) { throw 'Desktop UI ZIP contains duplicate member paths.' }
        $parts = $name.Split('/')
        if ($parts.Count -lt 2 -and -not $directory) {
            throw 'Desktop UI ZIP requires one containing root directory.'
        }
        [void]$roots.Add($parts[0])
        $attributes = $entry.ExternalAttributes
        if ((($attributes -shr 16) -band 0xf000) -eq 0xa000 -or
            ($attributes -band 0x400) -ne 0) {
            throw 'Desktop UI ZIP contains a symbolic link or reparse point.'
        }
    }
    if ($roots.Count -ne 1) { throw 'Desktop UI ZIP requires exactly one root directory.' }
} finally { $zip.Dispose() }

Expand-Archive -LiteralPath $archive -DestinationPath $destination
$rootEntries = @(Get-ChildItem -LiteralPath $destination -Force)
if ($rootEntries.Count -ne 1 -or -not $rootEntries[0].PSIsContainer) {
    throw 'Desktop UI ZIP has an unexpected root layout.'
}
$root = $rootEntries[0].FullName
$prefix = $root.TrimEnd('\') + '\'
$allEntries = @(Get-ChildItem -LiteralPath $root -Recurse -Force)
foreach ($item in $allEntries) {
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw 'Extracted desktop package contains a reparse point.'
    }
}
foreach ($required in @('Sovra UI.exe', 'application.exe', 'app.json', 'integrity.json',
    'ui/host.ps1', 'ui/renderer.ps1', 'ui/worker.ps1')) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $required) -PathType Leaf)) {
        throw "Desktop UI package is missing: $required"
    }
}

$manifest = Get-Content -LiteralPath (Join-Path $root 'integrity.json') -Raw | ConvertFrom-Json
if ($manifest.schema -ne 1) { throw 'Desktop UI integrity schema is unsupported.' }
$actual = @{}
foreach ($item in $allEntries) {
    if (-not $item.PSIsContainer) {
        $relative = $item.FullName.Substring($prefix.Length).Replace('\', '/')
        if ($relative -cne 'integrity.json') { $actual[$relative] = $item.FullName }
    }
}
$recordedPaths = [Collections.Generic.List[string]]::new()
$recordedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($record in @($manifest.files)) {
    if ($record.path -isnot [string] -or $record.sha256 -isnot [string] -or
        $record.sha256 -cnotmatch '^[0-9a-f]{64}$') {
        throw 'Desktop UI integrity file record is invalid.'
    }
    Assert-SafeRelativePath $record.path
    if (-not $recordedNames.Add($record.path)) { throw 'Desktop UI integrity repeats a file path.' }
    $recordedPaths.Add($record.path)
}
[string[]]$actualPaths = @($actual.Keys)
[string[]]$expectedPaths = $recordedPaths.ToArray()
[Array]::Sort($actualPaths, [StringComparer]::Ordinal)
[Array]::Sort($expectedPaths, [StringComparer]::Ordinal)
if ($actualPaths.Length -ne $expectedPaths.Length) { throw 'Desktop UI integrity file list differs.' }
for ($index = 0; $index -lt $actualPaths.Length; $index++) {
    if ($actualPaths[$index] -cne $expectedPaths[$index]) { throw 'Desktop UI integrity file list differs.' }
}
# Hash only verified actual files, never paths assembled from untrusted records.
foreach ($record in @($manifest.files)) {
    $actualHash = (Get-FileHash -LiteralPath $actual[$record.path] -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($record.sha256 -cne $actualHash) { throw "Desktop UI integrity mismatch: $($record.path)" }
}
$profile = Get-Content -LiteralPath (Join-Path $root 'app.json') -Raw | ConvertFrom-Json
if ($profile.schema -ne 1 -or $profile.id -isnot [string] -or
    $profile.id -cnotmatch '^[a-z][a-z0-9-]{0,63}$' -or $profile.program -cne 'application.exe') {
    throw 'Desktop UI application profile is invalid.'
}
if ($profile.PSObject.Properties.Name -contains 'view_command') {
    if ($profile.view_command -cnotin @('ui-view', 'ui-layout-view')) { throw 'Desktop UI view command is invalid.' }
}
Write-Output "Verified desktop UI package: $root"
