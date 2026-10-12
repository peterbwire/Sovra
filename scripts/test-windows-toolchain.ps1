param([Parameter(Mandatory=$true)][string]$Package)
$ErrorActionPreference = 'Stop'
$packageRoot = [IO.Path]::GetFullPath($Package)
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("sovra-toolchain-" + [guid]::NewGuid().ToString('N'))
$prefix = Join-Path $testRoot 'installed'
$project = Join-Path $testRoot 'hello'
$taskSource = [IO.Path]::GetFullPath('dogfood\01-task-manager\main.svr')
New-Item -ItemType Directory -Path $testRoot | Out-Null
try {
    if ($packageRoot.EndsWith('.zip', [StringComparison]::OrdinalIgnoreCase)) {
        $checksum = "$packageRoot.sha256"
        if (-not (Test-Path -LiteralPath $checksum -PathType Leaf)) { throw 'Toolchain checksum file is missing.' }
        $line = (Get-Content -LiteralPath $checksum -Raw).Trim()
        $expected = "^([0-9a-fA-F]{64})  $([regex]::Escape([IO.Path]::GetFileName($packageRoot)))$"
        if ($line -notmatch $expected) { throw 'Invalid toolchain checksum file.' }
        $actual = (Get-FileHash -LiteralPath $packageRoot -Algorithm SHA256).Hash
        if ($actual -ine $Matches[1]) { throw 'Toolchain ZIP checksum mismatch.' }
        $unpacked = Join-Path $testRoot 'unpacked'
        Expand-Archive -LiteralPath $packageRoot -DestinationPath $unpacked
        $packageRoot = Join-Path $unpacked 'sovra-windows'
        if (-not (Test-Path -LiteralPath (Join-Path $packageRoot 'install.ps1'))) {
            $folders = @(Get-ChildItem -LiteralPath $unpacked -Directory)
            if ($folders.Count -ne 1) { throw 'Toolchain ZIP has an unexpected root layout.' }
            $packageRoot = $folders[0].FullName
        }
    }
    & (Join-Path $packageRoot 'install.ps1') -Prefix $prefix -NoPersistPath | Out-Null
    $shell = Join-Path $PSHOME 'powershell.exe'
    $originalPath = $env:Path
    try {
        $env:Path = "$(Join-Path $prefix 'bin');$env:SystemRoot\System32;$env:SystemRoot"
        & $shell -NoProfile -ExecutionPolicy Bypass -File scripts\test-installed-toolchain.ps1 -Prefix $prefix -Project $project -TaskSource $taskSource
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
