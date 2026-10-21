# Diagnose a Sovra development environment

Use the running compiler to inspect its environment:

```text
svr doctor
svr doctor --target js
svr doctor --target exe
svr doctor --target desktop
svr doctor --format json
```

The default interpreter target can pass without Node or Rust. JavaScript
execution requires Node 22+, native builds require Rust/Cargo 1.74+ and a matching
runtime payload, and the private desktop host needs compatible Windows PowerShell
and WPF. A missing optional tool is a warning. Failed required checks exit 1;
malformed arguments exit 2. Doctor prints remediation but never applies repairs.

Installed packages validate their complete integrity inventory. A source build
identifies itself and does not claim installed integrity. Copying `svr.exe` alone
is not a complete installation. If files are damaged, verify a fresh archive's
checksum and follow the installation instructions rather than mixing versions.

Doctor captures a small general-purpose Sovra compiler/interpreter self-test in
memory. It does not run your application, write a project, invoke a native build,
download tools or open a UI window. Probes use temporary output captures and can
report timeouts or policy blocks. It does not bypass those blocks.

For automation, use the [doctor JSON contract](../reference/doctor-json.md).
For native application acceptance, follow [native builds](native-builds.md);
doctor reports linker readiness as unproven until an actual build is tested.
Clean Windows deployment and application tests remain separate checks.
