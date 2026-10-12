param([Parameter(Mandatory=$true)][string]$Package)
$ErrorActionPreference = 'Stop'
function Assert-ToolchainChecksum([string]$archive, [string]$checksum) {
    if (-not (Test-Path -LiteralPath $checksum -PathType Leaf)) { throw 'Toolchain checksum file is missing.' }
    $line = (Get-Content -LiteralPath $checksum -Raw).Trim()
    $expected = "^([0-9a-fA-F]{64})  $([regex]::Escape([IO.Path]::GetFileName($archive)))$"
    if ($line -notmatch $expected) { throw 'Invalid toolchain checksum file.' }
    $declaredHash = $Matches[1]
    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    if ($actual -ine $declaredHash) { throw 'Toolchain ZIP checksum mismatch.' }
}
$packageRoot = [IO.Path]::GetFullPath($Package)
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("sovra-toolchain-" + [guid]::NewGuid().ToString('N'))
$prefix = Join-Path $testRoot 'installed'
$taskSource = [IO.Path]::GetFullPath('dogfood\01-task-manager\main.svr')
New-Item -ItemType Directory -Path $testRoot | Out-Null
try {
    if ($packageRoot.EndsWith('.zip', [StringComparison]::OrdinalIgnoreCase)) {
        $checksum = "$packageRoot.sha256"
        Assert-ToolchainChecksum $packageRoot $checksum
        $corruptDir = Join-Path $testRoot 'corrupt'
        New-Item -ItemType Directory -Path $corruptDir | Out-Null
        $corrupt = Join-Path $corruptDir ([IO.Path]::GetFileName($packageRoot))
        Copy-Item -LiteralPath $packageRoot -Destination $corrupt
        $bytes = [IO.File]::ReadAllBytes($corrupt)
        if ($bytes.Length -eq 0) { throw 'Toolchain ZIP is empty.' }
        $bytes[$bytes.Length - 1] = $bytes[$bytes.Length - 1] -bxor 1
        [IO.File]::WriteAllBytes($corrupt, $bytes)
        $rejectedCorruption = $false
        try { Assert-ToolchainChecksum $corrupt $checksum } catch {
            if ($_.Exception.Message -notmatch 'checksum mismatch') { throw }
            $rejectedCorruption = $true
        }
        if (-not $rejectedCorruption) { throw 'Corrupted toolchain ZIP was accepted.' }
        $unpacked = Join-Path $testRoot 'unpacked'
        Expand-Archive -LiteralPath $packageRoot -DestinationPath $unpacked
        $packageRoot = Join-Path $unpacked 'sovra-windows'
        if (-not (Test-Path -LiteralPath (Join-Path $packageRoot 'install.ps1'))) {
            $folders = @(Get-ChildItem -LiteralPath $unpacked -Directory)
            if ($folders.Count -ne 1) { throw 'Toolchain ZIP has an unexpected root layout.' }
            $packageRoot = $folders[0].FullName
        }
    }
    $recordedImports = Get-Content -LiteralPath (Join-Path $packageRoot 'imports.json') -Raw | ConvertFrom-Json
    $actualImports = & .\scripts\audit-windows-imports.ps1 (Join-Path $packageRoot 'bin\svr.exe') | ConvertFrom-Json
    if ($recordedImports.architecture -ne $actualImports.architecture -or
        (@($recordedImports.imports) -join ',') -ne (@($actualImports.imports) -join ',')) {
        throw 'Packaged PE import record differs from svr.exe.'
    }
    $bundledTask = Join-Path $packageRoot 'samples\task-manager\main.svr'
    if ((Get-FileHash -LiteralPath $bundledTask -Algorithm SHA256).Hash -ne
        (Get-FileHash -LiteralPath $taskSource -Algorithm SHA256).Hash) {
        throw 'Bundled Task Manager source differs from dogfood source.'
    }
    $invalidPe = Join-Path $testRoot 'invalid.exe'
    [IO.File]::WriteAllBytes($invalidPe, [byte[]]::new(128))
    $rejectedInvalidPe = $false
    try { & .\scripts\audit-windows-imports.ps1 $invalidPe | Out-Null } catch {
        if ($_.Exception.Message -notmatch 'Not an MZ') { throw }
        $rejectedInvalidPe = $true
    }
    if (-not $rejectedInvalidPe) { throw 'PE import auditor accepted invalid input.' }
    $incomplete = Join-Path $testRoot 'incomplete-package'
    New-Item -ItemType Directory -Path (Join-Path $incomplete 'bin') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $packageRoot 'bin\svr.exe') -Destination (Join-Path $incomplete 'bin')
    Copy-Item -LiteralPath (Join-Path $packageRoot 'install.ps1') -Destination $incomplete
    $incompleteTarget = Join-Path $testRoot 'incomplete-install'
    $rejectedIncomplete = $false
    try { & (Join-Path $incomplete 'install.ps1') -Prefix $incompleteTarget -NoPersistPath | Out-Null } catch {
        if ($_.Exception.Message -notmatch 'missing') { throw }
        $rejectedIncomplete = $true
    }
    if (-not $rejectedIncomplete -or (Test-Path -LiteralPath $incompleteTarget)) {
        throw 'Incomplete package created an installation.'
    }
    $tampered = Join-Path $testRoot 'tampered-package'
    Copy-Item -LiteralPath $packageRoot -Destination $tampered -Recurse
    Add-Content -LiteralPath (Join-Path $tampered 'templates\console\main.svr') -Value '// tampered'
    $tamperedTarget = Join-Path $testRoot 'tampered-install'
    $rejectedTampering = $false
    try { & (Join-Path $tampered 'install.ps1') -Prefix $tamperedTarget -NoPersistPath | Out-Null } catch {
        if ($_.Exception.Message -notmatch 'integrity mismatch') { throw }
        $rejectedTampering = $true
    }
    if (-not $rejectedTampering -or (Test-Path -LiteralPath $tamperedTarget)) {
        throw 'Installer accepted a modified package file.'
    }
    $insidePackage = Join-Path $packageRoot 'nested-install'
    $rejectedOverlap = $false
    try { & (Join-Path $packageRoot 'install.ps1') -Prefix $insidePackage -NoPersistPath | Out-Null } catch {
        if ($_.Exception.Message -notmatch 'overlap') { throw }
        $rejectedOverlap = $true
    }
    if (-not $rejectedOverlap -or (Test-Path -LiteralPath $insidePackage)) {
        throw 'Installer accepted a prefix inside its package.'
    }
    & (Join-Path $packageRoot 'install.ps1') -Prefix $prefix -NoPersistPath | Out-Null
    $installedHash = (Get-FileHash -LiteralPath (Join-Path $prefix 'bin\svr.exe') -Algorithm SHA256).Hash
    $rejectedOverwrite = $false
    try { & (Join-Path $packageRoot 'install.ps1') -Prefix $prefix -NoPersistPath | Out-Null } catch {
        if ($_.Exception.Message -notmatch 'already exists') { throw }
        $rejectedOverwrite = $true
    }
    if (-not $rejectedOverwrite) { throw 'Installer overwrote an existing prefix.' }
    if ((Get-FileHash -LiteralPath (Join-Path $prefix 'bin\svr.exe') -Algorithm SHA256).Hash -ne $installedHash) {
        throw 'Rejected reinstall changed the compiler.'
    }
    $shell = Join-Path $PSHOME 'powershell.exe'
    $originalPath = $env:Path
    try {
        $env:Path = "$(Join-Path $prefix 'bin');$env:SystemRoot\System32;$env:SystemRoot"
        & $shell -NoProfile -ExecutionPolicy Bypass -File scripts\test-installed-toolchain.ps1 -Prefix $prefix -TaskSource $taskSource
        if ($LASTEXITCODE -ne 0) { throw 'Restricted-PATH installed toolchain test failed.' }
    } finally {
        $env:Path = $originalPath
    }
} finally {
    # This script owns the unique temp directory; never remove a caller path.
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    $resolved = [IO.Path]::GetFullPath($testRoot)
    if (-not $resolved.StartsWith($tempRoot + '\sovra-toolchain-', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove an unexpected test path.'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
