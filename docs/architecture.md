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

## Planned third-party library architecture

Sovra must support libraries published by independent developers. Package
selection, module loading and lexical symbol resolution are separate layers;
resolved module identities include the owning package identity. Imports remain
contained within each resolved package root, while declared dependencies allow
access to other authorized package roots. Applications consume public exports,
not all discovered source declarations.

The current project-local import checker is an initial implementation, not a
permanent single-project restriction. Package manifests, lockfiles, local/Git/
registry dependencies and publishing remain planned. See
[packages and libraries](design/PACKAGES_AND_LIBRARIES.md) for requirements,
implementation order, unresolved decisions and acceptance criteria.

