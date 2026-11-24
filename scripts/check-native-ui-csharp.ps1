param(
    [Parameter(Mandatory=$true)][string]$ReferenceDirectory,
    [string]$Output = 'target/native-ui-compile'
)
$ErrorActionPreference = 'Stop'
# Optional cross-host compiler check. Run using PowerShell 7 (Roslyn included)
# and Microsoft's .NET Framework 4.8 reference assemblies. Never load WPF on Linux.
Add-Type -Path (Join-Path $PSHOME 'Microsoft.CodeAnalysis.dll')
Add-Type -Path (Join-Path $PSHOME 'Microsoft.CodeAnalysis.CSharp.dll')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$outputPath = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $outputPath 'verification.json'), '{"schema_version":1,"success":false,"native_windows_executed":false}')
$referencePath = [IO.Path]::GetFullPath($ReferenceDirectory)
foreach ($name in @('mscorlib.dll','PresentationFramework.dll','PresentationCore.dll','WindowsBase.dll','System.Web.Extensions.dll')) {
    if (-not (Test-Path -LiteralPath (Join-Path $referencePath $name) -PathType Leaf)) { throw "Missing framework reference: $name" }
}
$references = [Collections.Generic.List[Microsoft.CodeAnalysis.MetadataReference]]::new()
foreach ($name in @('mscorlib.dll','System.dll','System.Core.dll','System.Xml.dll','System.Xaml.dll','System.Web.Extensions.dll','WindowsBase.dll','PresentationFramework.dll','PresentationCore.dll','UIAutomationClient.dll','UIAutomationTypes.dll')) {
    $references.Add([Microsoft.CodeAnalysis.MetadataReference]::CreateFromFile((Join-Path $referencePath $name)))
}
$sources = @('Bridge.cs','DesktopWindow.cs','DesktopTheme.cs','ApplicationProfile.cs','ProfileLease.cs','ProcessLifetime.cs','InvocationWorker.cs') |
    ForEach-Object { Join-Path $root ('toolchain/ui/windows/compiled/' + $_) }
$parseOptions = [Microsoft.CodeAnalysis.CSharp.CSharpParseOptions]::new([Microsoft.CodeAnalysis.CSharp.LanguageVersion]::CSharp5)
$results = @()
foreach ($kind in @('host','tests')) {
    $paths = @($sources)
    if ($kind -eq 'host') { $paths += Join-Path $root 'toolchain/ui/windows/compiled/Entry.cs' }
    else { $paths += Join-Path $root 'scripts/fixtures/CompiledDesktopTests.cs' }
    $trees = [Collections.Generic.List[Microsoft.CodeAnalysis.SyntaxTree]]::new()
    foreach ($path in $paths) {
        $text = [Microsoft.CodeAnalysis.Text.SourceText]::From([IO.File]::ReadAllText($path))
        $trees.Add([Microsoft.CodeAnalysis.CSharp.CSharpSyntaxTree]::ParseText($text, $parseOptions, $path))
    }
    $outputKind = [Microsoft.CodeAnalysis.OutputKind]::WindowsApplication
    if ($kind -eq 'tests') { $outputKind = [Microsoft.CodeAnalysis.OutputKind]::ConsoleApplication }
    $options = [Microsoft.CodeAnalysis.CSharp.CSharpCompilationOptions]::new($outputKind).WithPlatform([Microsoft.CodeAnalysis.Platform]::X64)
    $compilation = [Microsoft.CodeAnalysis.CSharp.CSharpCompilation]::Create('SovraNativeUi' + $kind, $trees, $references, $options)
    $stream = [IO.File]::Open((Join-Path $outputPath ($kind + '.exe')), [IO.FileMode]::Create)
    try { $result = $compilation.Emit($stream) } finally { $stream.Dispose() }
    if (-not $result.Success) { throw (($result.Diagnostics | ForEach-Object { $_.ToString() }) -join "`n") }
    $hashes = @($paths | ForEach-Object { [pscustomobject]@{ path = $_; sha256 = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant() } })
    $results += [pscustomobject]@{ kind = $kind; compiled = $true; sources = $hashes }
}
[IO.File]::WriteAllText((Join-Path $outputPath 'verification.json'), ([pscustomobject]@{
    schema_version = 1; kind = 'native-ui-reference-compilation'; success = $true;
    language_version = 'CSharp5'; framework = '.NET Framework 4.8'; platform = 'x64'; native_windows_executed = $false; results = $results
} | ConvertTo-Json -Depth 5))
Write-Output 'PASS native UI and acceptance fixture compile as C# 5 against .NET Framework/WPF references; native execution not performed.'
