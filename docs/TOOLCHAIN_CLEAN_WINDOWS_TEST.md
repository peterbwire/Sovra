# Clean Windows toolchain acceptance

Status: **Not yet executed**. This procedure qualifies an exact downloaded
Windows toolchain artifact, not a developer checkout or restricted-PATH process.
Use a compatible, disposable Windows image with no Sovra, Rust/Cargo or Node.js
installed. Record the image version and architecture before testing. Do not
reuse a machine with an existing Sovra user PATH entry or installation.

Copy `sovra-windows.zip` and `sovra-windows.zip.sha256` from the same CI run to
the image. In PowerShell, from their directory:

```powershell
$line = (Get-Content .\sovra-windows.zip.sha256 -Raw).Trim()
$actual = (Get-FileHash .\sovra-windows.zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($line -ne "$actual  sovra-windows.zip") { throw 'ZIP checksum mismatch' }
Expand-Archive .\sovra-windows.zip -DestinationPath .\unpacked
Set-Location .\unpacked\sovra-windows
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
```

Close that PowerShell window. Open a new one, then run:

```powershell
Get-Command svr.exe | Select-Object -ExpandProperty Source
svr.exe --version
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\Sovra\verify-install.ps1"
```

The self-test creates, checks, runs and tests a console project, runs a bundled local
dependency example, and checks/adds/lists/completes/deletes tasks using the
bundled Task Manager CLI source. It uses a unique temporary directory and
removes that directory afterward. The GUI is not part of this Node-free test.
Check that `Get-Command svr.exe` points to the newly installed `bin` directory
and that the self-test exits 0. Also record the contents of `imports.json` and
whether all named DLLs resolve on the image. The file lists direct PE imports
only; dynamic and transitive dependencies require separate observation.

Capture the ZIP SHA-256, CI run/commit, Windows edition/build/architecture,
install path, command outputs, exit codes, and any missing-DLL or application
error. Preserve this evidence in release qualification. A successful hosted
Windows CI run or a developer-machine test does not substitute for this run.
Supported minimum Windows release and CRT policy remain open until the exact
artifact passes on the selected image and those facts are documented.
