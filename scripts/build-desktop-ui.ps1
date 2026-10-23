param(
    [Parameter(Mandatory=$true)][string]$Program,
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string]$AppId,
    [switch]$GroupedLayout,
    [switch]$CompiledHost
)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$application = [IO.Path]::GetFullPath($Program)
$stage = [IO.Path]::GetFullPath($Output)
if ($AppId -cnotmatch '^[a-z][a-z0-9-]{0,63}$') { throw 'Invalid application data identity.' }
if (-not (Test-Path -LiteralPath $application -PathType Leaf)) { throw 'Build the native Sovra application first.' }
if ((Test-Path -LiteralPath $stage) -or (Test-Path -LiteralPath "$stage.zip")) { throw 'Desktop output already exists.' }
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (-not (Test-Path -LiteralPath $csc -PathType Leaf)) { throw 'The Windows .NET Framework compiler is unavailable.' }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
$adapter = Join-Path $repository 'toolchain\ui\windows'
if (-not $CompiledHost) {
    New-Item -ItemType Directory -Path (Join-Path $stage 'ui') | Out-Null
    Copy-Item -LiteralPath (Join-Path $adapter 'host.ps1'), (Join-Path $adapter 'renderer.ps1'), (Join-Path $adapter 'worker.ps1') -Destination (Join-Path $stage 'ui')
}
Copy-Item -LiteralPath $application -Destination (Join-Path $stage 'application.exe')
if ($CompiledHost) {
    Copy-Item -LiteralPath (Join-Path $adapter 'compiled\README.md') -Destination $stage
} else {
    Copy-Item -LiteralPath (Join-Path $adapter 'README.md') -Destination $stage
}
Copy-Item -LiteralPath (Join-Path $repository 'LICENSE-MIT'), (Join-Path $repository 'LICENSE-APACHE') -Destination $stage
if ($CompiledHost) {
    $wpf = Join-Path (Split-Path -Parent $csc) 'WPF'
    $sources = @('Bridge.cs', 'DesktopWindow.cs', 'DesktopTheme.cs', 'ApplicationProfile.cs', 'Entry.cs') |
        ForEach-Object { Join-Path $adapter ('compiled\' + $_) }
    & $csc /nologo /target:winexe /platform:x64 /optimize+ /reference:System.Web.Extensions.dll /reference:System.Xaml.dll `
        "/reference:$wpf\PresentationFramework.dll" "/reference:$wpf\PresentationCore.dll" "/reference:$wpf\WindowsBase.dll" `
        "/out:$(Join-Path $stage 'Sovra UI.exe')" $sources
} else {
    & $csc /nologo /target:winexe /platform:x64 /optimize+ /reference:System.Windows.Forms.dll "/out:$(Join-Path $stage 'Sovra UI.exe')" (Join-Path $adapter 'launcher.cs')
}
if ($LASTEXITCODE -ne 0) { throw 'Desktop launcher compilation failed.' }
$utf8 = [Text.UTF8Encoding]::new($false)
$profile = [pscustomobject]@{ schema = 1; id = $AppId; program = 'application.exe' }
if ($GroupedLayout) { $profile | Add-Member -NotePropertyName view_command -NotePropertyValue 'ui-layout-view' }
[IO.File]::WriteAllText((Join-Path $stage 'app.json'), ($profile | ConvertTo-Json), $utf8)
$prefix = $stage.TrimEnd('\') + '\'
$files = @(Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
    [pscustomobject]@{
        path = $_.FullName.Substring($prefix.Length).Replace('\', '/')
        sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
} | Sort-Object path)
[IO.File]::WriteAllText((Join-Path $stage 'integrity.json'),
    ([pscustomobject]@{ schema = 1; files = $files } | ConvertTo-Json -Depth 4), $utf8)
Compress-Archive -LiteralPath $stage -DestinationPath "$stage.zip"
$hash = (Get-FileHash -LiteralPath "$stage.zip" -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText("$stage.zip.sha256", "$hash  $([IO.Path]::GetFileName($stage)).zip`n", [Text.Encoding]::ASCII)
Write-Output "Built private desktop UI package: $stage.zip"
