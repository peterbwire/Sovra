# Experimental single-entry packages

```sh
svr check --experimental-v2 --format json path/to/app
svr run --experimental-v2 path/to/app -- first-argument
svr test --experimental-v2 --format json path/to/app
svr build --experimental-v2 --emit ir path/to/app
```

These modes accept either a regular `.svr` file or an existing package directory
with `sovra.toml`. A supported package has exactly one root entry and no dependency
aliases or registered module files. Entries may live in nested directories, such
as `src/main.svr`. The compiler checks the captured entry as a complete v2 root
program: enums, records, aliases, linked functions and recursive match/control
bodies retain their existing rules. It does not use the application scanner as
proof of semantic validity. See [the example](../../examples/enum-package/README.md).

The package resolver retains its graph, manifest, path, source-size and identity
admission. After resolution, dependencies/registered modules refuse with E3124
before entry-byte capture. Invalid graphs or unsafe/over-budget registrations may
fail earlier under existing package diagnostics. There is no partial dependency
linking, import fallback or directory scan for extra `.svr` files. Unregistered
files are not loaded. Inline modules/imports in the entry remain unsupported by
the v2 root checker.

Entry bytes use the existing guarded reader: regular-file and opened-handle
identity checks, bounded strict UTF-8 reads, before/after metadata checks and
Linux procfs containment validation. The compiler consumes the returned captured
text without rereading the entry path. Exact manifest bytes used by resolution
are retained by the capture API. This is not an atomic whole-filesystem snapshot.
Current guarded capture supports Linux x86/x86_64/aarch64 with procfs; other hosts
refuse rather than use a weaker reader. Portable source-file v2 commands remain
available independently; additional package-host qualification remains open.

Check JSON keeps schema 1 and the existing directory kind `project`; test JSON
keeps its existing `package` kind. Both retain the input target as supplied and
place original root-source byte spans under the canonical entry filename.
Manifest/path errors preserve manifest provenance. Global quota failures retain
unknown/null locations. Human output escapes control bytes. Standalone source
report kinds are unchanged.

`run` still requires parameterless Unit `main`; check, test and IR inspection do
not require it. Test discovery remains entry-local and does not automatically run
main. IR inspection is byte-for-byte identical to inspection of the same entry
as a standalone source. Runtime/suite/output quotas and default host denial are
unchanged. Program arguments still follow `--`. Package selection does not change
the process working directory: granted file services use the caller's CWD and
permissions, not implicit package-root authority. Input/file grants are still
[explicit experimental-run options](experimental-v2-hosts.md).

Optional [locked v2 commands](experimental-v2-locks.md) now verify source/manifest
integrity before compilation or effects. Unlocked mode still reads no lock and
never modifies it. Standalone capture alone does not claim locking, atomic
snapshots or semantic validity. Supported-directory configuration/source/runtime
failures return 1; existing successful command statuses remain unchanged.

Rust callers may use `packages::capture_root_source` and inspect its immutable
`CapturedRootSource` inventory/manifests/entry/source before passing captured bytes
to `v2::check_program_source`. Capture alone does not parse or check source and
cannot establish a checked or locked package. `into_source` transfers entry/text
without another filesystem read.

G13/E1 remains partial: dependencies/imports, module visibility and dependency lock qualification, IDE
package contexts, JS/native backends and cross-platform package acceptance remain
open. Default commands and existing package/compiler APIs keep their behavior.

V2 also supports [source-backed native executable builds](experimental-v2-native.md)
through `--emit exe --output PATH`, with optional package `--locked`. This does not
close dependency/module linking or executable IR codec qualification.
