# Sovra Windows toolchain (first installable slice)

This package contains `bin/svr.exe`, the Rust-owned compiler/interpreter and
standard library, a console project template, documentation, and a user-scope
PowerShell installer. No Node or Rust installation is needed to **check or run**
the included template. Native application building remains an internal proof
and currently requires Rust/Cargo on the developer machine.

From an extracted package, run in PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
```

Open a new PowerShell window, then:

```powershell
svr --version
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\Sovra\bin\svr-new.ps1" .\hello
svr check .\hello\main.svr
svr run .\hello\main.svr
```

`svr-new.ps1` refuses to overwrite an existing directory. `install.ps1` uses
`%LOCALAPPDATA%\Programs\Sovra` and adds its `bin` directory to the current
user's PATH. Use `-Prefix <directory>` for an alternate location. The installer
refuses to overwrite an existing prefix; remove an old installation manually
before reinstalling. PowerShell's execution policy may require the explicit
`-ExecutionPolicy Bypass -File` invocation for `svr-new.ps1` as well.

`svr new`, `svr test`, `svr fmt`, `svr install` and `svr update` are not
implemented. `svr build` currently emits human-readable IR or Node JavaScript,
not a native application. Local dependency packages can be checked and run;
registry publishing and lockfiles are not available.
