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

