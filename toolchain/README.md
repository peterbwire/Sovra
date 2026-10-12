# Sovra Windows toolchain (first installable slice)

This package contains `bin/svr.exe`, the Rust-owned compiler/interpreter and
standard library, a console project template, repository documentation and
examples, and a user-scope PowerShell installer. No Node or Rust installation
is needed to **check or run**
the included template or bundled Task Manager CLI sample. Native application
building remains an internal proof
and currently requires Rust/Cargo on the developer machine.
`imports.json` records the compiler's direct Windows DLL imports; it does not
include transitive or dynamically loaded dependencies.
`integrity.json` lists SHA-256 hashes for package files. The installer checks
this list before copying, so damaged or locally modified extracted files are
rejected. The manifest is included in the ZIP, not independently signed; verify
the ZIP's published checksum when obtaining a release.

From an extracted package, run in PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
```

Open a new PowerShell window, then:

```powershell
svr --version
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\Sovra\verify-install.ps1"
svr new .\hello
svr check .\hello\main.svr
svr run .\hello
```

`verify-install.ps1` is contained in the extracted package and uses a unique
temporary project. It checks that PATH resolves the installed `svr.exe`, then
creates, checks and runs the template, bundled local-package example and Task
Manager CLI sample without
the Sovra repository, Rust or Node. Pass `-Prefix <directory>` if you installed
outside the default path. The bundled docs retain links into compiler source
and dogfood files in the repository; those development sources are not part of
the installed toolchain. Some bundled examples describe planned application
syntax; only examples marked executable in their documentation are runnable.
The `samples/task-manager/main.svr` file is copied unchanged from the dogfood
application; see its adjacent README for direct commands. Its browser GUI is
not part of this Node-free compiler package.

`svr new` refuses to overwrite an existing path. `svr-new.ps1` delegates to
that command for older installation scripts. `install.ps1` uses
`%LOCALAPPDATA%\Programs\Sovra` and adds its `bin` directory to the current
user's PATH. Use `-Prefix <directory>` for an alternate location. The installer
validates the package before copying, stages files beside the final prefix,
and refuses to overwrite an existing prefix. Remove an old installation
manually before reinstalling. PowerShell's execution policy may require the explicit
`-ExecutionPolicy Bypass -File` invocation for `svr-new.ps1` as well.

`svr test`, `svr fmt`, `svr install` and `svr update` are not
implemented. `svr build` currently emits human-readable IR or Node JavaScript,
not a native application. Local dependency packages can be checked and run;
registry publishing and lockfiles are not available.

The compiler source is in the repository's `src/` tree: `src/main.rs` is the
entry point, `src/cli.rs` handles commands, `src/project_new.rs` implements
project creation, and `src/compiler/` contains the language pipeline and
Rust-owned standard library. `toolchain/` holds distribution assets, rather
than a second copy of compiler source. The package builder compiles those Rust
sources into `bin/svr.exe` and adds the installer, templates, samples and docs.
