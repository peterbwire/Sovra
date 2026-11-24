# Private installer operations, shared with explicit rollback. PowerShell 5.1+.
function Resolve-SovraInstallDirectory([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    $separator = [IO.Path]::DirectorySeparatorChar
    if ($full.TrimEnd($separator) -eq [IO.Path]::GetPathRoot($full).TrimEnd($separator)) { throw 'Installation prefix cannot be a filesystem root.' }
    return $full.TrimEnd($separator)
}
function Get-SovraInstallHash([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $algorithm = [Security.Cryptography.SHA256]::Create()
        try { return [BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
        finally { $algorithm.Dispose() }
    } finally { $stream.Dispose() }
}

function Assert-SovraInstallPackage([string]$Path) {
    $root = [IO.Path]::GetFullPath($Path)
    $item = Get-Item -LiteralPath $root -Force
    if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Installation root must be a real directory.' }
    $manifestPath = Join-Path $root 'integrity.json'
    $manifestItem = Get-Item -LiteralPath $manifestPath -Force
    if ($manifestItem.Length -gt 8388608 -or ($manifestItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Invalid installation integrity manifest.' }
    $manifest = [IO.File]::ReadAllText($manifestPath, [Text.UTF8Encoding]::new($false, $true)) | ConvertFrom-Json
    if ($manifest.schema -ne 1 -or @($manifest.files).Count -gt 20000) { throw 'Unsupported installation integrity manifest.' }
    $expected = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
    $unique = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $manifest.files) {
        if ($entry.path -isnot [string] -or $entry.sha256 -isnot [string] -or
            $entry.path -match '(^/|\\|:|(^|/)\.\.?(/|$)|//)' -or
            -not $entry.path -or $entry.sha256 -cnotmatch '^[0-9a-f]{64}$' -or -not $unique.Add($entry.path)) {
            throw 'Invalid or duplicate installation integrity entry.'
        }
        $expected.Add($entry.path, $entry.sha256)
    }
    if (-not $expected.ContainsKey('bin/svr.exe')) { throw 'Directory is not a manifest-owned Sovra installation.' }
    $separator = [string][IO.Path]::DirectorySeparatorChar
    $prefix = $root.TrimEnd([IO.Path]::DirectorySeparatorChar) + $separator
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($root)
    $count = 0
    while ($pending.Count -gt 0) {
        foreach ($file in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
            if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Installation contains a reparse point.' }
            if ($file.PSIsContainer) { $pending.Push($file.FullName); continue }
            $relative = $file.FullName.Substring($prefix.Length).Replace($separator, '/')
            if ($relative -ceq 'integrity.json') { continue }
            if (-not $expected.ContainsKey($relative)) { throw "Unmanaged installation file: $relative" }
            if ((Get-SovraInstallHash $file.FullName) -cne $expected[$relative]) { throw "Package integrity mismatch: $relative" }
            $count++
        }
    }
    if ($count -ne $expected.Count) { throw 'Installation integrity file list mismatch.' }
}

function Get-SovraInstallLeasePath([string]$Target) {
    $path = Resolve-SovraInstallDirectory $Target
    if ([IO.Path]::DirectorySeparatorChar -eq '\') { $path = $path.ToUpperInvariant() }
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try { $key = [BitConverter]::ToString($algorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($path))).Replace('-', '').ToLowerInvariant() }
    finally { $algorithm.Dispose() }
    return Join-Path ([IO.Path]::GetDirectoryName($path)) ('.sovra-install-' + $key + '.lock')
}

function New-SovraInstallRecoveryReceipt([string]$Target, [string]$Backup) {
    $receipt = [pscustomobject]@{ schema = 1; target = (Resolve-SovraInstallDirectory $Target); backup = (Resolve-SovraInstallDirectory $Backup); compiler_sha256 = (Get-SovraInstallHash (Join-Path $Target 'bin/svr.exe')) }
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($receipt | ConvertTo-Json))
    if ($bytes.Length -gt 16384) { throw 'Installation recovery receipt is too large.' }
    $stream = [IO.File]::Open(($Backup + '.recovery.json'), [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
}

function Assert-SovraInstallRecoveryReceipt([string]$Target, [string]$Backup) {
    $path = $Backup + '.recovery.json'
    $item = Get-Item -LiteralPath $path -Force
    if ($item.Length -gt 16384 -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Invalid installation recovery receipt.' }
    $receipt = [IO.File]::ReadAllText($path, [Text.UTF8Encoding]::new($false, $true)) | ConvertFrom-Json
    if ($receipt.schema -ne 1 -or $receipt.target -isnot [string] -or $receipt.backup -isnot [string] -or
        $receipt.compiler_sha256 -isnot [string] -or $receipt.compiler_sha256 -cnotmatch '^[0-9a-f]{64}$' -or
        (Get-SovraInstallLeasePath $receipt.target) -cne (Get-SovraInstallLeasePath $Target) -or
        (Get-SovraInstallLeasePath $receipt.backup) -cne (Get-SovraInstallLeasePath $Backup) -or
        $receipt.compiler_sha256 -cne (Get-SovraInstallHash (Join-Path $Backup 'bin/svr.exe'))) { throw 'Recovery receipt does not match the target and backup.' }
}

function Invoke-SovraInstallCommit {
    param([string]$Stage, [string]$Target, [switch]$Upgrade, [Parameter(Mandatory=$true)][scriptblock]$Validate)
    $stagePath = Resolve-SovraInstallDirectory $Stage
    $targetPath = Resolve-SovraInstallDirectory $Target
    $parent = [IO.Path]::GetDirectoryName($targetPath)
    if (-not $parent -or $stagePath -eq $targetPath -or [IO.Path]::GetDirectoryName($stagePath) -ne $parent -or
        [IO.Path]::GetFileName($stagePath) -cnotmatch '^\.sovra-(install|backup)-[0-9a-f]{32}$') { throw 'Unsafe installation staging path.' }
    $lease = [IO.File]::Open((Get-SovraInstallLeasePath $targetPath), [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    $backup = $null
    $published = $false
    try {
        Assert-SovraInstallPackage $stagePath
        if ([IO.Path]::GetFileName($stagePath).StartsWith('.sovra-backup-', [StringComparison]::Ordinal)) { Assert-SovraInstallRecoveryReceipt $targetPath $stagePath }
        & $Validate $stagePath
        if (Test-Path -LiteralPath $targetPath) {
            if (-not $Upgrade) { throw "Install prefix already exists: $targetPath" }
            Assert-SovraInstallPackage $targetPath
            $backup = Join-Path $parent ('.sovra-backup-' + [guid]::NewGuid().ToString('N'))
            New-SovraInstallRecoveryReceipt $targetPath $backup
            [IO.Directory]::Move($targetPath, $backup)
        }
        try {
            # Directory.Move refuses existing destinations rather than nesting.
            [IO.Directory]::Move($stagePath, $targetPath)
            $published = $true
            Assert-SovraInstallPackage $targetPath
            & $Validate $targetPath
        } catch {
            $failure = $_
            try {
                if ($published) { [IO.Directory]::Move($targetPath, $stagePath) }
                if ($backup) { [IO.Directory]::Move($backup, $targetPath); $backup = $null }
            } catch { throw "Installation recovery incomplete. Preserve target=$targetPath stage=$stagePath backup=$backup. $($_.Exception.Message)" }
            throw $failure
        }
        return [pscustomobject]@{ Target = $targetPath; Backup = $backup }
    } finally {
        # Keep the inert lease file: deleting it would permit inode/name races.
        $lease.Dispose()
    }
}

function Test-SovraInstallCompiler([string]$Path) {
    $compiler = Join-Path $Path 'bin/svr.exe'
    $version = (& $compiler --version) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $version -notmatch '^svr [0-9]+\.[0-9]+\.[0-9]+$') { throw 'Staged compiler version check failed.' }
    $template = Join-Path $Path 'templates/console/main.svr'
    $json = (& $compiler check --format json $template) -join "`n"
    if ($LASTEXITCODE -ne 0 -or ($json | ConvertFrom-Json).success -ne $true) { throw 'Staged compiler source check failed.' }
}
