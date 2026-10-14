param([string]$Output = 'target\toolchain\runtime-source')
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$destination = [IO.Path]::GetFullPath($Output)
if (Test-Path -LiteralPath $destination) { throw "Output already exists: $destination" }
$parent = Split-Path -Parent $destination
New-Item -ItemType Directory -Path $parent -Force | Out-Null
$stage = Join-Path $parent ('.sovra-runtime-stage-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    foreach ($file in @('Cargo.toml','Cargo.lock','rustfmt.toml','LICENSE','LICENSE-MIT','LICENSE-APACHE')) {
        Copy-Item -LiteralPath (Join-Path $repository $file) -Destination $stage
    }
    Copy-Item -LiteralPath (Join-Path $repository 'src') -Destination $stage -Recurse
    New-Item -ItemType Directory -Path (Join-Path $stage 'examples') | Out-Null
    Copy-Item -LiteralPath (Join-Path $repository 'examples\native_builder.rs') `
        -Destination (Join-Path $stage 'examples')
    New-Item -ItemType Directory -Path (Join-Path $stage 'toolchain\templates') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repository 'toolchain\templates\console') `
        -Destination (Join-Path $stage 'toolchain\templates') -Recurse
    Copy-Item -LiteralPath (Join-Path $repository 'toolchain\runtime-source\README.md') `
        -Destination (Join-Path $stage 'README.md')
    New-Item -ItemType Directory -Path (Join-Path $stage 'toolchain\runtime-source') | Out-Null
    Copy-Item -LiteralPath (Join-Path $repository 'toolchain\runtime-source\runner.rs') `
        -Destination (Join-Path $stage 'toolchain\runtime-source')
    $prefix = $stage.TrimEnd('\') + '\'
    $files = @(Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
        [pscustomobject]@{
            path = $_.FullName.Substring($prefix.Length).Replace('\', '/')
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    } | Sort-Object path)
    $manifest = [pscustomobject]@{ schema = 1; kind = 'sovra-native-runtime-source'; files = $files }
    [IO.File]::WriteAllText((Join-Path $stage 'payload.json'),
        ($manifest | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    $version = [regex]::Match((Get-Content -LiteralPath (Join-Path $stage 'Cargo.toml') -Raw),
        '(?m)^version = "([^"]+)"').Groups[1].Value
    $hashLines = @("SVR-RUNTIME 1 $version") + @($files | ForEach-Object { "$($_.sha256)  $($_.path)" })
    [IO.File]::WriteAllText((Join-Path $stage 'payload.sha256'),
        (($hashLines -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
    if (Test-Path -LiteralPath $destination) { throw "Output already exists: $destination" }
    Move-Item -LiteralPath $stage -Destination $destination
    Write-Output "Built internal native runtime source payload: $destination"
} finally {
    # Only this invocation's reserved directory can be removed.
    $stagePath = [IO.Path]::GetFullPath($stage)
    $parentPrefix = [IO.Path]::GetFullPath($parent).TrimEnd('\') + '\'
    if (-not $stagePath.StartsWith($parentPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Unsafe runtime staging cleanup path.'
    }
    if (Test-Path -LiteralPath $stagePath) { Remove-Item -LiteralPath $stagePath -Recurse -Force }
}
