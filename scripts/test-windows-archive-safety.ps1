param([switch]$IntegrationOnly)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-zip-safety-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
$checks = 0
function New-Fixture([string]$Path, [string[]]$Names, [int]$Attributes = 0) {
    $zip = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $Names) {
            $entry = $zip.CreateEntry($name)
            $entry.ExternalAttributes = $Attributes
        }
    } finally { $zip.Dispose() }
    $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$Path.sha256", "$hash  $([IO.Path]::GetFileName($Path))`n", [Text.UTF8Encoding]::new($false))
}
try {
    # All integration payloads remain inside owned extraction/temp areas even
    # if the regression returns. They contain no executable/script payloads.
    $archive = Join-Path $scratch 'traversal.zip'
    New-Fixture $archive @('root/../escaped.txt')
    foreach ($verifier in @('test-windows-toolchain.ps1', 'test-native-task-manager-artifact.ps1', 'test-desktop-package.ps1')) {
        $destination = Join-Path $scratch $verifier.Replace('.ps1', '')
        $rejected = $false
        try {
            if ($verifier -eq 'test-windows-toolchain.ps1') {
                & (Join-Path $PSScriptRoot $verifier) -Package $archive | Out-Null
            } elseif ($verifier -eq 'test-native-task-manager-artifact.ps1') {
                & (Join-Path $PSScriptRoot $verifier) -Package $archive -ExtractTo $destination | Out-Null
            } else {
                & (Join-Path $PSScriptRoot $verifier) -PackageZIP $archive -ExtractTo $destination -CompiledHost | Out-Null
            }
        } catch {
            if ($_.Exception.Message -notmatch 'ZIP contains an unsafe relative path') { throw "$verifier did not reject during member preflight: $($_.Exception.Message)" }
            $rejected = $true
        }
        if (-not $rejected -or (Test-Path -LiteralPath $destination)) { throw "$verifier extracted an unsafe ZIP." }
        $checks++
    }
    if (-not $IntegrationOnly) {
        . (Join-Path $PSScriptRoot 'assert-windows-zip.ps1')
        $cases = @(
            @('root/../outside'), @('/absolute/file'), @('C:/absolute/file'),
            @('root//empty'), @('root/./dot'), @('root/trailing.'), @('root/trailing '),
            @('root/a:stream'), @('root/CON.txt'), @('root/LPT1'), @('root/a?b'),
            @('root/file', 'root/FILE'), @('root/item', 'root/item/child'),
            @('root/dir/file', 'root/DIR'), @('one/a', 'two/b'), @('root-file')
            @('root\..\outside'), @('//server/share/file'), @('root/file', 'root/file/'),
            @('Root/a', 'root/b')
        )
        $index = 0
        foreach ($names in $cases) {
            $path = Join-Path $scratch ("invalid-$index.zip"); $index++
            New-Fixture $path $names
            $rejected = $false
            try { Assert-WindowsZipMembers $path | Out-Null } catch { $rejected = $true }
            if (-not $rejected) { throw "Preflight accepted invalid members: $($names -join ', ')" }
            $checks++
        }
        foreach ($attributes in @([int]0xa0000000, 0x400, [int]0x10000000)) {
            $path = Join-Path $scratch ("attributes-$attributes.zip")
            New-Fixture $path @('root/link') $attributes
            $rejected = $false
            try { Assert-WindowsZipMembers $path | Out-Null } catch { $rejected = $true }
            if (-not $rejected) { throw 'Preflight accepted link/reparse/special member metadata.' }
            $checks++
        }
        foreach ($item in @(
            @{ name = 'directory-type.zip'; path = 'root/file'; attributes = 0x40000000 },
            @{ name = 'file-type.zip'; path = 'root/dir/'; attributes = [int]0x80000000 }
        )) {
            $path = Join-Path $scratch $item.name
            New-Fixture $path @($item.path) $item.attributes
            $rejected = $false
            try { Assert-WindowsZipMembers $path | Out-Null } catch { $rejected = $true }
            if (-not $rejected) { throw 'Preflight accepted conflicting entry type/path metadata.' }
            $checks++
        }
        $empty = Join-Path $scratch 'empty.zip'
        New-Fixture $empty @()
        $rejected = $false
        try { Assert-WindowsZipMembers $empty | Out-Null } catch { $rejected = $true }
        if (-not $rejected) { throw 'Preflight accepted an empty archive.' }
        $checks++
        $valid = Join-Path $scratch 'valid.zip'
        New-Fixture $valid @('root/', 'root/bin/', 'root/bin/svr.exe', 'root\docs\README.md')
        if ((Assert-WindowsZipMembers $valid) -cne 'root') { throw 'Preflight rejected a valid Windows archive.' }
        $checks++
    }
    Write-Output "PASS Windows ZIP preflight: $checks checks."
} finally {
    $expected = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\sovra-zip-safety-'
    $owned = [IO.Path]::GetFullPath($scratch)
    if (-not $owned.StartsWith($expected, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe ZIP fixture cleanup path.' }
    Remove-Item -LiteralPath $owned -Recurse -Force
}
