# Sovra architecture

Sovra keeps the language definition, compiler, runtime, standard library,
tooling, and `svr` command separate so each layer can evolve independently.

The initial Rust crate exposes the compiler-stage boundaries under
`src/compiler/`. M0 only establishes those boundaries and the CLI contract.
M1 will add tokens and lexing; later milestones can connect parsing, semantic
analysis, interpretation, and code generation without changing the public
repository shape.

See [docs/architecture.md](docs/architecture.md) for the pipeline and
[docs/roadmap.md](docs/roadmap.md) for milestone sequencing.
