# Sovra architecture

Sovra keeps the language definition, compiler, runtime, standard library,
tooling, and `svr` command separate so each layer can evolve independently.

The compiler, runtime, and core tooling are implemented in Rust. The Rust crate
exposes the compiler-stage boundaries under `src/compiler/`.
The M0-M11 foundation includes lexing, parsing, AST construction, semantic
analysis, interpretation, modules, core types, a small `std` namespace,
inspectable IR, and a portable JavaScript compiler backend.

Over time, stable parts of the standard library and ecosystem can be written in
Sovra itself. That migration should happen at the library boundary: the trusted
compiler, runtime, diagnostics, package/project tooling, and bootstrap path
remain Rust-owned until Sovra has enough self-hosting support to carry them
without weakening reliability.

See [docs/architecture.md](docs/architecture.md) for the pipeline and
[docs/roadmap.md](docs/roadmap.md) for milestone sequencing.
