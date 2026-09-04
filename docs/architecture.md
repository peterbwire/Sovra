# Architecture

`svr` is organized as a pipeline, with each stage isolated behind a module:

```text
source -> lexer -> parser -> ast -> semantic -> ir -> backend
                                      \-> diagnostics
                         ir -> interpreter
```

The current M11 pipeline has working lexer, parser, AST, semantic analysis,
standard-library registry, IR lowering, text IR rendering, portable JavaScript
backend rendering, and interpreter execution stages. Stages receive explicit
inputs and return structured results. Error reporting belongs in `diagnostics`,
rather than ad-hoc output in individual stages. The CLI remains a thin adapter
over the compiler API.

The compiler, runtime, and core tooling are implemented in Rust. Future Sovra
releases may move stable standard-library modules, packages, templates, and
ecosystem tools into Sovra itself, but those pieces should sit on top of the
Rust implementation boundary until the language has a deliberate self-hosting
story.

