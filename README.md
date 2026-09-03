# Sovra

Sovra is a small, expressive programming language and toolchain. This
repository contains the M4 foundation: a stable command-line entry point,
compiler pipeline boundaries, a lexer, parser, semantic validation, a minimal
intermediate representation, an interpreter, and a text backend.

## Quick start

```text
cargo run -- --version
cargo run -- --help
cargo test
```

The canonical executable is `svr`. M4 supports `run` and `build` for a source
path; remaining commands are listed by `svr --help` and report a clear,
non-zero “not implemented” message when selected.

The first source example is [`examples/hello-world/main.svr`](examples/hello-world/main.svr).
Run it with `cargo run -- run examples/hello-world/main.svr`.

## Repository layout

* `src/` — CLI and compiler-stage modules
* `examples/hello-world/` — first Sovra source example
* `docs/` — specification, architecture, roadmap, and contributor guidance
* `.github/workflows/` — CI for formatting, linting, and tests

Read [docs/spec.md](docs/spec.md) for the M0 scope, [ARCHITECTURE.md](ARCHITECTURE.md)
for the system boundaries, and [CONTRIBUTING.md](CONTRIBUTING.md) for
development conventions.

## License

Sovra is dual-licensed under either the MIT license or the Apache License,
Version 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
