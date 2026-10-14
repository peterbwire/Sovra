# Native Task Manager qualification artifact

This executable is built from the Sovra Task Manager CLI through the approved
`svr build --emit exe --output PATH` command. It embeds checked IR and the Rust
interpreter. This archive awaits clean-machine release qualification. The
browser GUI is not included.

Run `task-manager.exe help` to see CLI commands. Tasks are stored in the text
file path supplied to each command. To verify this package on Windows without
Node, Rust or Sovra installed, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\verify.ps1
```

`task-manager.exe.sha256` records the binary hash; `imports.json` lists direct
Windows DLL imports. `build.json` records the public build method, compiler version/hash,
source hash and architecture for the exact artifact; these are provenance
records, not a publisher signature. Verify the separately supplied ZIP checksum before
extracting a downloaded artifact. This self-test does not establish a minimum
supported Windows version or prove that transitive/dynamic DLL dependencies
are available on every machine.
