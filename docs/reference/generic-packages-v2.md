# Generic root packages and source locks

Explicit generic `check`, `run` and native `build` now accept a directory with
one manifest-selected entry and no dependencies or registered modules. They use
the [guarded root capture](experimental-v2-packages.md), not the application scanner
or a directory search for `.svr` files. Unregistered files are not compiled.
This is single-entry project support; reusable dependency/module linking remains
open.

For example, from a repository checkout on a supported Linux host:

```sh
mkdir -p target/generic-demo/src
cp docs/examples/generic-source.svr target/generic-demo/src/main.svr
```

Create `target/generic-demo/sovra.toml`:

```toml
[project]
name = "generic"
entry = "src/main.svr"
```

Then use the current compiler:

```sh
svr check --experimental-generics --format json target/generic-demo
svr run --experimental-generics target/generic-demo
svr lock --experimental-generics target/generic-demo
svr lock --experimental-generics --check target/generic-demo
svr check --experimental-generics --locked --format json target/generic-demo
svr build --experimental-generics --locked --emit exe --output target/generic-app target/generic-demo
```

The example verifies a generic record/enum/function result and exits silently.
Rust/Cargo and a host linker are build-time requirements; the
[native executable](generic-native-v2.md) needs neither the project nor Node.js
at runtime. Existing standalone source command behavior remains available.

Capture validates graph, manifest, path and source ownership using the existing
bounded resolver and guarded opened-handle reader. Generic commands select a
4 MiB source-read ceiling through `capture_root_source_with_source_limit` before
retaining entry bytes. This additive Rust API only reduces existing frontend
admission; zero never selects unlimited reading. Existing `capture_root_source`
keeps its prior default. Manifest/graph/name budgets remain in force. Generic
compilation/specialization continues to use the same finite single-source profile;
this is not a linked package graph with independent per-file allowances.

The captured original source and canonical entry filename move into compilation
without rereading source paths. `--locked` verifies those same captured manifest
and source bytes before compilation, execution or native output creation. Missing,
stale, malformed, unsafe or oversized locks refuse without fallback or lock
updates. Unlocked commands neither read nor write a lock. `lock` generates or
checks the existing version-one source-lock schema, using the existing guarded
publisher. A source fingerprint establishes integrity against recorded bytes,
not semantic validity, authentication or an atomic filesystem transaction. The
lock command deliberately does not type-check or execute source.

Check JSON retains schema 1, the supplied target and kind `project`. Known source
errors use the canonical entry file and original byte/line/column coordinates;
manifest errors retain manifest provenance and unknown global failures use null
locations. Generic compile/entry errors remain E3123. Runtime errors now use
E3125, correcting the earlier experimental E3124 collision: E3124 remains the
existing refusal for unsupported dependency/module package shapes. Lock errors
retain E4165 and read-budget errors retain E1003. Human output is escaped.

Package capture currently requires supported Linux x86/x86_64/aarch64 and procfs;
other hosts refuse rather than falling back to a weaker reader. Standalone source
commands remain separate. Generic entry syntax still refuses imports and inline
modules. Package selection does not grant stdin, output, files, network, arguments
or a package working directory. Runtime policy and parameterless Unit-main
requirements remain unchanged. Native images retain only bounded source, with
their separate 1 MiB ceiling; no project locator, lock, grants or mutable inputs
are loaded by the generated program.

Duplicate `--locked`, standalone locked inputs and incompatible engine/permission
flags are usage errors. Generic test, formatter, IDE, dependency/module linking,
JavaScript and standard error/propagation integration remain open. This does not
complete G13 E2, package ecosystem work or other-host qualification.

`tests/generic_package_v2.rs` covers real generic package check/run/native build,
source locks, canonical diagnostics, unused-file handling, exact reduced source
admission, unsupported shapes/imports, link refusal and independent native
execution after removing all package inputs. Existing v2 package/lock tests
verify that the shared command parser and default capture keep their behavior.
