# Sovra architecture

Sovra keeps the language definition, compiler, runtime, standard library,
tooling, and `svr` command separate so each layer can evolve independently.

The Rust crate exposes the compiler-stage boundaries under `src/compiler/`.
The M0-M11 foundation includes lexing, parsing, AST construction, semantic
analysis, interpretation, modules, core types, a small `std` namespace,
inspectable IR, and a portable JavaScript compiler backend.

See [docs/architecture.md](docs/architecture.md) for the pipeline and
[docs/roadmap.md](docs/roadmap.md) for milestone sequencing.
