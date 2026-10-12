param(
    [Parameter(Mandatory=$true, Position=0)][string]$Path
)
$ErrorActionPreference = 'Stop'
$destination = [IO.Path]::GetFullPath($Path)
$name = [IO.Path]::GetFileName($destination.TrimEnd('\', '/'))
if ($name -notmatch '^[A-Za-z][A-Za-z0-9_-]*$') {
    throw 'Project directory name must start with a letter and contain only letters, digits, _ or -.'
}
if (Test-Path -LiteralPath $destination) { throw "Project path already exists: $destination" }
$template = Join-Path $PSScriptRoot '..\templates\console'
if (-not (Test-Path -LiteralPath (Join-Path $template 'main.svr'))) {
    throw 'Installed Sovra console template is missing.'
}
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $template 'main.svr') -Destination $destination
$manifest = Get-Content -LiteralPath (Join-Path $template 'sovra.toml') -Raw
$manifest = $manifest.Replace('name = "console-app"', "name = `"$name`"")
[IO.File]::WriteAllText((Join-Path $destination 'sovra.toml'), $manifest, [Text.UTF8Encoding]::new($false))
Write-Output "Created $destination"
