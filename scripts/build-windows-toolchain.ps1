param(
    [string]$Binary = 'target\release\svr.exe',
    [string]$Output = 'target\toolchain\sovra-windows'
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) { throw 'This package is for Windows.' }
$compiler = [IO.Path]::GetFullPath($Binary)
$stage = [IO.Path]::GetFullPath($Output)
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) { throw "Build svr.exe first: $compiler" }
if (Test-Path -LiteralPath $stage) { throw "Output already exists: $stage" }
$archive = "$stage.zip"
if (Test-Path -LiteralPath $archive) { throw "Archive already exists: $archive" }
New-Item -ItemType Directory -Path (Join-Path $stage 'bin') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $stage 'docs') -Force | Out-Null
Copy-Item -LiteralPath $compiler -Destination (Join-Path $stage 'bin\svr.exe')
Copy-Item -LiteralPath 'toolchain\bin\svr-new.ps1' -Destination (Join-Path $stage 'bin')
Copy-Item -LiteralPath 'toolchain\install.ps1' -Destination $stage
Copy-Item -LiteralPath 'toolchain\templates' -Destination $stage -Recurse
Copy-Item -LiteralPath 'docs\spec.md','docs\reference\check-json.md','docs\course\local-packages.md' -Destination (Join-Path $stage 'docs')
Copy-Item -LiteralPath 'toolchain\README.md' -Destination (Join-Path $stage 'README.md')
Copy-Item -LiteralPath 'LICENSE','LICENSE-MIT','LICENSE-APACHE' -Destination $stage
Compress-Archive -LiteralPath $stage -DestinationPath $archive
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $([IO.Path]::GetFileName($archive))" | Set-Content -Encoding ascii "$archive.sha256"
Write-Output "Staged $stage"
