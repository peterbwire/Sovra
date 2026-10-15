# Build a standalone CLI application

Under accepted ADR 0027, `svr build --emit exe` links checked Sovra source or a
local dependency package with the Rust interpreter into a host-native executable.
The developer needs compatible Rust/Cargo and a host linker. End users do not
need Node, Rust, the source files or the Sovra compiler installed.

```powershell
svr new .\hello
svr check .\hello\main.svr
svr test .\hello
svr build --emit exe --output .\hello.exe .\hello\main.svr
.\hello.exe
```

For a local dependency package, pass its directory instead of a source file.
The package's entry and declared local dependencies use the existing checker
and linker; private members and nominal record identities retain their rules.
The executable uses its launch working directory for relative file paths and
passes its own arguments to `std::arg_count`/`std::arg`.

```powershell
svr build --emit exe --output .\task-manager.exe .\task-manager.svr
.\task-manager.exe add .\tasks.txt "Ship a tested application"
.\task-manager.exe list .\tasks.txt
```

`--output` is required. Windows output must end in `.exe`. Existing files,
directories and links are refused. Linking happens in owned staging beside
the output; publication cannot overwrite a file that appears during linking.
Successful builds print the final executable path. Source/payload/link errors
return status 1; invalid options return 2. Failed stages are cleaned.

Installed `svr` requires its version-matched `runtime-source` snapshot beside
`bin`. File list, hashes and version are checked before Cargo runs. Missing or
damaged snapshots fail rather than falling back to a checkout. The snapshot's
hashes detect damage relative to its manifest, not publisher authenticity;
verify the toolchain archive checksum when installing. Development binaries
under this repository's `target` can use the repository's trusted source.

This first slice builds for the host. It does not support `--target` or custom
Cargo cross-target configuration. Filesystems must support same-filesystem
hard links for no-overwrite publication. Cross-compilation, optimized native
code for each Sovra function and a stable runtime ABI remain later work.

Existing default IR and `--emit js` output remain on stdout. `--output` is only
accepted with `--emit exe`. Clean compatible Windows deployment and supported
host qualification remain release gates even when local tests pass.
