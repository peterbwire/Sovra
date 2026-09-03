# Architecture

`svr` is organized as a pipeline, with each stage isolated behind a module:

```text
source -> lexer -> parser -> ast -> semantic -> ir -> backend
                                      \-> diagnostics
                         ir -> interpreter
```

M0 provides the module boundaries and CLI contract only. Stages should receive
explicit inputs and return structured results as they are implemented. Error
reporting belongs in `diagnostics`, rather than ad-hoc output in individual
stages. The CLI should remain a thin adapter over the compiler API.

