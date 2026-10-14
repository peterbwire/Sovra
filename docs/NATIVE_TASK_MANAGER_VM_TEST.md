# Native Task Manager clean Windows test

Status: **Pending user VM test.** This checks a qualification artifact built
through public `svr build --emit exe --output PATH`, embedding checked IR and
the Rust interpreter. Local artifact checks do not establish release readiness.
The browser GUI is not included.

Copy `sovra-task-manager.zip` and `sovra-task-manager.zip.sha256` from the same
Windows CI run to a clean compatible Windows VM without Node, Rust/Cargo or
Sovra installed. Record the CI commit, Windows version/build and architecture.
In PowerShell, from the directory containing both files:

```powershell
$expected = (Get-Content .\sovra-task-manager.zip.sha256 -Raw).Trim()
$actual = (Get-FileHash .\sovra-task-manager.zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expected -cne "$actual  sovra-task-manager.zip") { throw 'ZIP checksum mismatch' }
Expand-Archive .\sovra-task-manager.zip -DestinationPath .\unpacked
Set-Location .\unpacked\sovra-task-manager
powershell -NoProfile -ExecutionPolicy Bypass -File .\verify.ps1
```

`verify.ps1` checks the executable hash, runs add/list/complete/delete across
separate processes, and confirms malformed state is rejected without rewriting
it. It uses a unique temporary directory and removes that directory afterward.
For a separate persistent-state check, run:

```powershell
.\task-manager.exe add .\tasks.txt "VM task"
.\task-manager.exe list .\tasks.txt
.\task-manager.exe complete .\tasks.txt "VM task"
.\task-manager.exe list .\tasks.txt
.\task-manager.exe delete .\tasks.txt "VM task"
```

Keep `build.json` and `imports.json` with the recorded results. The build record
identifies the compiler version/hash, source hash and public build method.
The import record lists direct DLL imports but
does not establish that all transitive or dynamically loaded dependencies are
available. Record each command's output and exit status, any missing-DLL or
application error, and the executable and ZIP hashes. A passing developer-host
or hosted-CI test does not replace this VM test.
