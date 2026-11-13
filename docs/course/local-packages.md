# Consuming a local Sovra library

Implemented scope: local path dependencies, one executable entry file per package,
ordinary inline modules, exported functions and exported records. No registry or installation step
is involved. The trusted compiler remains implemented in Rust.

Compilation permits at most 16 MiB per entry file and 64 MiB total entry sources
across the canonical dependency graph, including the root. Shared dependencies
and multiple aliases to one package count once. UTF-8 comments/whitespace count
toward the byte limits. Each file also has a 1,000,000-token limit and a
256-diagnostic lexer/parser cap. Aggregate overflow reports E1003 at the entry
that exceeds the remaining allowance; compilation and package tests stop before
running any code. See [ADR 0035](../adr/0035-frontend-resource-budgets.md).

Package loading also permits at most 1 MiB per manifest, 8 MiB total canonical
manifest bytes and 8,192 dependency aliases. A shared package's manifest counts
once; separate aliases still consume separate edges. E4112 identifies an excess
manifest or alias header, before resolving the over-budget target. These limits
also protect the initial project-classification manifest read; see
[ADR 0036](../adr/0036-package-graph-resource-budgets.md).

Interface string materialization permits 128 MiB cumulatively across the graph.
Copied signature names/types, exported/attached records, generated aliases and
qualified signature labels count at their UTF-8 byte lengths. Repeated imports
consume the allowance again even when replacing earlier metadata. E3024 stops
compilation before successful linking or execution; see accepted
[ADR 0039](../adr/0039-interface-string-materialization.md). This is a component
quota rather than a complete compiler memory/CPU bound.

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

The arithmetic-only snippet prints `49`. The repository example additionally
constructs an exported record and prints `25` and `1.5`.
The CLI regression also executes the emitted JavaScript
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

## Exported records

A library can declare a public nominal record inside its module:

```svr
mod geometry {
    export struct Point { x: Float, y: Float }
}
```

After `use shapes::geometry;`, consumers can use `shapes::geometry::Point`
in parameter, return and local annotations, and construct it with
`shapes::geometry::Point { x: 3, y: 4 }`. Read fields with `point.x`.
All fields must be supplied exactly once; compatible Int values widen to Float.
Unexported records remain private across packages, including in public signatures
and public field types (E4116). Equal names and layouts in different packages
never make records interchangeable. Two aliases for the same package preserve
the same record identity. Scalar aliases resolve in the library.

All exported record fields are readable and constructible in this milestone.
Field mutation is not added. Future explicit field visibility and encapsulation
remain possible under a compatibility design; see [ADR 0011](../adr/0011-exported-record-interfaces.md).

## Limits

This is mutable local-source consumption, not a lockfile or integrity guarantee.
No Git/registry downloads, publishing, installation scripts or package-manager
commands have been added. Only package entry files are compiled; general multi-file
source loading and re-exports remain planned. Inline `mod`/`export fn` syntax is
unchanged. Generics, exported type aliases, recursive-type support and HTTP remain
unfinished. Module-local type names are isolated: multiple library modules can
declare `Point` or `Scalar` independently. Exported signatures and fields resolve
against their owner's types, never a same-named type in another module or consumer.
An imported function explicitly returning `Float` preserves Float inference for
mutable locals and array elements, including compatible Int-to-Float widening.
Scalar aliases in exported signatures resolve in the library, so a consumer
need not redeclare them. A same-named consumer alias cannot reinterpret them.
Exported records cross package interfaces while preserving nominal identity.

CLI directory checking selects executable package compilation when dependency
sections are present. Directories without them still use the partial application
checker; check a standalone library's entry `.svr` directly for source semantics.
`run` and `build` accept package directories even without dependencies. Experimental
`--service-calls` is for application scanning and is rejected for dependency packages.


General multi-file registration and local-source locks are now covered by a
[approved G12 design](../design/G12_MODULES_AND_LOCKS.md). The additive
[source-inventory API](../reference/package-source-inventory.md) and
[source-unit loader](../reference/package-source-units.md) currently admit module
registrations for metadata/captured parsing. Semantic linking, locks and locked commands are not
available in this lesson's implemented entry-only workflow.


A separate [multi-file callable example](../../examples/multi-file-packages/README.md)
now executes through the additive Rust compiler API with primitive interfaces and
private helpers. Its manifests still require that API; the default CLI workflow
above remains entry-only, and general record interfaces/locks remain planned.

A separate [registered shapes/record example](../../examples/multi-file-records/README.md)
now executes nominal primitive-field records through the record-capable Rust API.
It does not enable default CLI module loading or lock workflows.


## Registered sources through the CLI

On the supported guarded Linux reader, both [callable](../../examples/multi-file-packages/README.md)
and [record](../../examples/multi-file-records/README.md) multi-file examples now
support CLI check/run/build and root test discovery. See the [registered CLI
contract](../reference/registered-package-cli.md) for commands and remaining
context/formatter/editor limitations. Older Rust package APIs keep their original
entry-only contract; locks remain planned.
