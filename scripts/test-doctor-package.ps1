param([Parameter(Mandatory=$true)][string]$Package)
$ErrorActionPreference = 'Stop'
$source = [IO.Path]::GetFullPath($Package)
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('sovra-doctor-package-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
$originalPath = $env:Path
$junction = $null
try {
    $copy = Join-Path $scratch 'installed package'
    Copy-Item -LiteralPath $source -Destination $copy -Recurse
    $compiler = Join-Path $copy 'bin\svr.exe'
    $env:Path = "$env:SystemRoot\System32;$env:SystemRoot"
    function Read-Health([bool]$ExpectedSuccess, [string]$Program = $compiler) {
        $output = (& $Program doctor --format json) -join "`n"
        $status = $LASTEXITCODE
        $health = $output | ConvertFrom-Json
        $expectedCode = if ($ExpectedSuccess) { 0 } else { 1 }
        if ($status -ne $expectedCode -or $health.schema_version -ne 1 -or $health.kind -ne 'doctor' -or
            $health.success -ne $ExpectedSuccess) { throw "Unexpected doctor result: $output" }
        return $health
    }
    $healthy = Read-Health $true
    if (@($healthy.checks | Where-Object { $_.id -eq 'install.integrity' -and $_.required -and $_.status -eq 'pass' }).Count -ne 1) {
        throw 'Packaged compiler did not validate its actual installation.'
    }
    $readme = Join-Path $copy 'README.md'
    $bytes = [IO.File]::ReadAllBytes($readme)
    [IO.File]::AppendAllText($readme, 'tampered')
    $null = Read-Health $false
    [IO.File]::WriteAllBytes($readme, $bytes)
    Remove-Item -LiteralPath $readme
    $null = Read-Health $false
    [IO.File]::WriteAllBytes($readme, $bytes)
    $extra = Join-Path $copy 'unexpected.txt'
    [IO.File]::WriteAllText($extra, 'extra')
    $null = Read-Health $false
    Remove-Item -LiteralPath $extra
    $manifest = Join-Path $copy 'integrity.json'
    $manifestBytes = [IO.File]::ReadAllBytes($manifest)
    Remove-Item -LiteralPath $manifest
    $null = Read-Health $false
    [IO.File]::WriteAllBytes($manifest, $manifestBytes)
    foreach ($invalid in @('traversal','duplicate')) {
        $data = [Text.Encoding]::UTF8.GetString($manifestBytes) | ConvertFrom-Json
        if ($invalid -eq 'traversal') { $data.files[0].path = '../outside' }
        else { $data.files = @($data.files) + @($data.files[0]) }
        [IO.File]::WriteAllText($manifest, ($data | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
        $null = Read-Health $false
    }
    [IO.File]::WriteAllBytes($manifest, $manifestBytes)
    $outside = Join-Path $scratch 'outside'
    New-Item -ItemType Directory -Path $outside | Out-Null
    $junction = Join-Path $copy 'linked-directory'
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
    $null = Read-Health $false
    [IO.Directory]::Delete($junction) # Delete only the junction, never its target.
    $junction = $null
    $loose = Join-Path $scratch 'bare'
    New-Item -ItemType Directory -Path $loose | Out-Null
    $bareCompiler = Join-Path $loose 'svr.exe'
    Copy-Item -LiteralPath $compiler -Destination $bareCompiler
    $null = Read-Health $false $bareCompiler
    $null = Read-Health $true
    Write-Output 'PASS packaged doctor: restricted PATH, integrity, tamper/missing/extra files, unsafe/duplicate entries, junction and relocated bare EXE.'
} finally {
    $env:Path = $originalPath
    if ($junction -and (Test-Path -LiteralPath $junction)) {
        if (((Get-Item -LiteralPath $junction -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) {
            throw 'Refusing cleanup of an unexpected junction path.'
        }
        [IO.Directory]::Delete($junction)
    }
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    $owned = [IO.Path]::GetFullPath($scratch)
    if (-not $owned.StartsWith($tempRoot + '\sovra-doctor-package-', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing cleanup outside the owned doctor fixture.'
    }
    Remove-Item -LiteralPath $owned -Recurse -Force
}
