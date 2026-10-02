# Consuming a local Sovra library

Implemented scope: local path dependencies, one executable entry file per package,
ordinary inline modules and exported functions. No registry or installation step
is involved. The trusted compiler remains implemented in Rust.

The [consumer](../../examples/local-packages/app/main.svr) declares this dependency
in its [manifest](../../examples/local-packages/app/sovra.toml):

```toml
[project]
name = "library-consumer"
version = "1.0.0"
entry = "main.svr"

[dependencies.utilities]
path = "../utilities"
```

Paths are relative to this manifest. The sibling library has its own manifest
and an entry file containing `mod arithmetic`. Its `square` function is exported;
its `product` helper is private.

```svr
use utilities::arithmetic;

fn main() {
    std::println(utilities::arithmetic::square(7))
}
```

Run from the repository root:

```text
cargo run -- check examples/local-packages/app
cargo run -- check --format=json examples/local-packages/app
cargo run -- run examples/local-packages/app
cargo run -- build examples/local-packages/app
cargo run -- build --emit js examples/local-packages/app
```

Execution prints `49`. The CLI regression also executes the emitted JavaScript
with Node. Pass the package directory: passing `app/main.svr` selects standalone
source checking, which does not load a dependency manifest.

Imports require `use alias::module;`, including the semicolon. Calls remain fully
qualified as `alias::module::function`. Duplicate identical imports are harmless.
Only direct declared dependencies and explicitly imported modules are visible.
Transitive packages can serve their own callers but cannot be imported by your
package without a direct dependency declaration. Private functions remain visible
only in their defining module. All loaded package entry bodies are validated,
including private helpers and unused dependencies; dependency `main` functions
are not executed. Library top-level helpers cannot capture consumer names.

The compiler rejects package cycles, missing modules/dependencies, malformed
manifests, unknown types, wrong arguments and canonical entry/manifest escapes.
Declared sibling packages are permitted. Equal display names do not merge packages;
canonical local roots define graph identity, while aliases define source spelling.

## Limits

This is mutable local-source consumption, not a lockfile or integrity guarantee.
No Git/registry downloads, publishing, installation scripts or package-manager
commands have been added. Only package entry files are compiled; general multi-file
source loading and re-exports remain planned. Inline `mod`/`export fn` syntax is
unchanged. Other declaration forms, collections, generics and HTTP remain outside
the executable subset.

CLI directory checking selects executable package compilation when dependency
sections are present. Directories without them still use the partial application
checker; check a standalone library's entry `.svr` directly for source semantics.
`run` and `build` accept package directories even without dependencies. Experimental
`--service-calls` is for application scanning and is rejected for dependency packages.
