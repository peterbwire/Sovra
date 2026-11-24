# Sovra Windows toolchain (first installable slice)

This package contains `bin/svr.exe`, the Rust-owned compiler/interpreter and
standard library, a console project template, repository documentation and
examples, and a user-scope PowerShell installer. The course lives only in the
main Sovra repository and is excluded from this package. Links to course lessons
inside bundled documents refer to that repository, not offline package files. No Node or Rust installation
is needed to **check or run**
the included template or bundled Task Manager CLI sample. Native application
building uses `svr build --emit exe --output PATH` and requires Rust/Cargo
and a compatible host linker on the developer machine.

Optional editor, LSP and extension sources ship in `developer-tools/`, with an
installed `bin/svr-editor.ps1` launcher. These tools require Node.js 22+; normal
compiler/application execution does not. Run `svr-editor -Check`, then
`svr-editor` and open its printed URL. Use `-NodePath` for Node outside PATH.
For LSP clients, spawn Node directly with installed `developer-tools/launch.mjs`
and argument `lsp` to preserve binary stdio. See
[installed developer tools](developer-tools/README.md) for paths and limits.
CI-built toolchains include a source-verified VSIX for manual installation;
source builds include it when `build-windows-toolchain.ps1 -IdeExtension` is
supplied. Packaging requires Node; nothing installs Node or VS Code automatically.

The `runtime-source/` snapshot contains the trusted source and internal native
builder needed to link without a Sovra checkout. Its README describes the
source snapshot and internal proof invocation. The installed public command
validates this snapshot before linking and fails clearly if it is missing or damaged.
The installed self-test also verifies that a blocked Task Manager save preserves
its data and another writer's marker, then resumes after deliberate recovery of
the test's own marker. It does not remove or recover locks in user profiles.
It also checks record alias identity, nested-array Float widening and preservation
of the original Int array through actual installed compiler execution.
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

To replace an existing installation, close its compiler/editor/tooling processes
and run the installer from the newly extracted, checksum-verified package:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Upgrade
```

Use the same `-Prefix` as the installation being replaced. Without `-Upgrade`,
existing prefixes are still refused. The installer verifies both package trees,
checks the candidate compiler/template before and after activation, and retains
the old directory as `.sovra-backup-<id>` beside the prefix. Modified files,
unmanaged files or links in the existing installation block replacement. Keep
projects/profiles outside the toolchain prefix; no user data is migrated or deleted.
Failed activation restores the prior directory when possible and retains the
candidate for inspection. Backups and failed candidates are never automatically
deleted. Allow disk space for both versions.

The installer prints the retained backup path. To restore it, run from an
extracted package containing the rollback helper:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\rollback.ps1 -Backup 'C:\path\.sovra-backup-<id>'
```

Pass `-Prefix` for a custom installation. Rollback verifies the backup, its
`.recovery.json` sibling receipt and any current installation; the displaced
version is retained as another backup. Receipt target/hash checks prevent an
accidental restore to another prefix. Receipts are coordination records, not
publisher signatures. If an interrupted installer left the prefix missing,
explicit rollback can restore the backup identified by its receipt. Never guess
or automatically select a backup; preserve all reported paths if recovery fails.

Cooperating installers share a per-prefix exclusive file lease; its inert lock
file remains beside the installation and does not block recovery after a crash.
Do not delete lease files during installation. Directory replacement and receipt
flushing are not a power-loss durability guarantee or a Windows registry
transaction. PATH integration follows successful activation; a PATH-update error
can leave a valid installed version and its retained backup. Existing unrelated
user PATH text is preserved, and rollback uses the same prefix without editing it.
Native Windows/PowerShell 5.1 upgrade and clean-machine qualification remain CI/
release gates; portable transaction tests alone do not satisfy them.
See the [G05 qualification checklist](../docs/INSTALLER_QUALIFICATION.md) for
portable evidence and the remaining distinct-version/clean-machine procedure.

Open a new PowerShell window, then:

```powershell
svr --version
svr doctor
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\Sovra\verify-install.ps1"
svr new .\hello
svr check .\hello\main.svr
svr run .\hello
svr test .\hello
```

`verify-install.ps1` is contained in the extracted package and uses a unique
temporary project. It checks that PATH resolves the installed `svr.exe`, then
validates installed integrity through `svr doctor`,
creates, checks and runs the template, bundled local-package example and Task
Manager CLI sample without
the Sovra repository, Rust or Node. Pass `-Prefix <directory>` if you installed
outside the default path. The bundled docs retain links into compiler source
and dogfood files in the repository; the runtime snapshot is a build-only subset,
not a full compiler development checkout. Some bundled examples describe planned application
syntax; only examples marked executable in their documentation are runnable.
The `samples/task-manager/main.svr` file is copied unchanged from the dogfood
application; see its adjacent README for direct commands. Its browser GUI is
not part of this Node-free compiler package.

`svr doctor --target js|exe|desktop` checks the selected target's prerequisites;
`--format json` produces a versioned report. Interpreter mode does not require
Node or Rust. Doctor reports problems and remediation without installing tools,
modifying projects or bypassing policy. Native linking and clean-machine support
remain unproven until their separate acceptance tests pass. See
`docs/reference/doctor-json.md` for output and probe limits.

`svr new` refuses to overwrite an existing path. `svr-new.ps1` delegates to
that command for older installation scripts. Failed generation rolls back files
it created, preserves colliding/unrelated files and only removes an empty reserved
directory. It does not lock the directory against concurrent writers.
`install.ps1` uses
`%LOCALAPPDATA%\Programs\Sovra` and adds its `bin` directory to the current
user's PATH. Use `-Prefix <directory>` for an alternate location. The installer
validates the package before copying, stages files beside the final prefix,
and refuses to overwrite an existing prefix. Remove an old installation
manually before reinstalling. PowerShell's execution policy may require the explicit
`-ExecutionPolicy Bypass -File` invocation for `svr-new.ps1` as well.

`svr test` runs root-level parameterless Unit test functions with human or JSON
reports; the template includes a test. `svr fmt [--check | --write] file.svr`
formats standalone source without Node/Rust; Windows writes are guarded and atomic.
Imports/application syntax are deferred; see `docs/reference/formatter.md`.
`svr install` and `svr update` are not implemented.
`svr build` emits human-readable IR, Node JavaScript or a host-native
executable with `--emit exe --output app.exe`. Local dependency packages can be checked and run;
registry publishing and lockfiles are not available.

The compiler source is in the repository's `src/` tree: `src/main.rs` is the
entry point, `src/cli.rs` handles commands, `src/project_new.rs` implements
project creation, and `src/compiler/` contains the language pipeline and
Rust-owned standard library. `toolchain/` holds distribution assets. The package
builder compiles those Rust sources into `bin/svr.exe` and generates a version-matched
`runtime-source/` snapshot, then adds the installer, templates, samples and docs.
