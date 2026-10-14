# Sovra architecture

Sovra keeps the language definition, compiler, runtime, standard library,
tooling, and `svr` command separate so each layer can evolve independently.

The compiler, runtime, and core tooling are implemented in Rust. The Rust crate
exposes the compiler-stage boundaries under `src/compiler/`.
The M0-M11 foundation includes lexing, parsing, AST construction, semantic
analysis, interpretation, modules, core types, a small `std` namespace,
inspectable IR, and a portable JavaScript compiler backend.

Local dependency packages resolve, check and link through the same compiler
pipeline. The Rust interpreter and process host own arguments, terminal I/O,
bounded UTF-8 files and exit status. The JavaScript backend uses a Node host.
The Windows toolchain packages these implemented capabilities with templates,
documentation and installation/self-test scripts.

The approved internal standalone proof serializes checked IR and embeds it
with the Rust interpreter in a native executable. Its private image decoder
validates declarations, instruction payloads, jumps and calls. The builder
checks the entry point before linking. Accepted ADR 0027 exposes host-native
linking through `svr build --emit exe --output PATH`, with validated installed
runtime source and atomic no-overwrite publication. Deployment
qualification remains outstanding; see
[Node-free application execution](docs/architecture/NODE_FREE_APPLICATIONS.md).

Over time, stable parts of the standard library and ecosystem can be written in
Sovra itself. That migration should happen at the library boundary: the trusted
compiler, runtime, diagnostics, package/project tooling, and bootstrap path
remain Rust-owned until Sovra has enough self-hosting support to carry them
without weakening reliability.

See [docs/architecture.md](docs/architecture.md) for the pipeline and
[docs/roadmap.md](docs/roadmap.md) for milestone sequencing.
