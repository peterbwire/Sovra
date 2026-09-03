# Sovra

Sovra is a small, expressive programming language and toolchain. This
repository contains the M0 foundation: a stable command-line entry point and
the compiler pipeline boundaries that later milestones will fill in.

## Quick start

```text
cargo run -- --version
cargo run -- --help
cargo test
```

The canonical executable is `svr`. M0 intentionally does not parse or execute
Sovra programs yet. Future commands are listed by `svr --help` and report a
clear, non-zero “not implemented” message when selected.

## Repository layout

* `src/` — CLI and compiler-stage modules
* `examples/hello-world/` — first Sovra source example
* `docs/` — specification, architecture, roadmap, and contributor guidance
* `.github/workflows/` — CI for formatting, linting, and tests

Read [docs/spec.md](docs/spec.md) for the M0 scope and [CONTRIBUTING.md](CONTRIBUTING.md)
for development conventions.

## License

Sovra is dual-licensed under either the MIT license or the Apache License,
Version 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

