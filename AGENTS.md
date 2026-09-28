# Sovra project context

Sovra (`svr`, `.svr`) is a general-purpose language and software ecosystem for
humans and AI agents. Prioritize correctness, coherent semantics, simplicity,
developer experience, machine-readable structure, then performance/ecosystem
breadth. Nova is a future AI engineering system; language foundations come first.
Business entities belong in libraries/examples, not core language primitives.

## Start here

Read `docs/CODEX_HANDOFF_ASSESSMENT.md`, `docs/spec.md`, `docs/roadmap.md`, and
`docs/production-upgrade-flow.md` before substantive work. Inspect Git status
and preserve existing changes. Continue the implementation incrementally;
do not restart it, silently change syntax, or reorganize it for appearance.

## Architecture and current milestone

Single Rust 2021 Cargo package `sovra`, binary `svr`; declared minimum Rust 1.74.
`src/main.rs` delegates to `src/cli.rs`. Public compiler stages are in
`src/compiler/`: lexer -> parser/AST -> semantic -> IR -> interpreter/backend,
with diagnostics and Rust-owned stdlib. `project.rs` is a separate manifest and
line-scanning checker, not full application parsing or type-checking.

The repository calls M0-M11 complete and M12 project checking in progress.
The assessment recommends existing-subset correctness hardening first. Module
bodies, numeric widening/overflow, token ranges and straight-line returns now
have regression coverage. ADR 0002 is approved and implemented: every function
parameter requires an explicit annotation (`E3014`); local `let` inference and
the default Unit return type remain. Named-type resolution is still incomplete.
Consult `docs/FULL_DEVELOPMENT_STATUS.md` and `docs/adr/` for active work and decisions.
Do not renumber the established milestones.
`run` executes one source file; `build` emits IR or JS; `check` accepts a source
file or project. `check --format json` emits versioned diagnostic reports; see
`docs/reference/check-json.md` for null locations and the project-scan scope.
Other commands, including Sovra `test`, are reserved.

`tests/cli.rs` covers subprocess behavior; unit tests sit beside Rust modules.
`examples/hello-world` is executable. `examples/fielddesk` is proposed application
syntax with partial wiring validation. Passing its check does not make it runnable.
`examples/modules` and `docs/course/modules.md` cover inline modules;
`examples/functions` and `docs/course/functions.md` cover typed signatures and
local inference. There is no separate runtime crate, package manager or language
server yet.

For library/package work, read `docs/LIBRARY_ECOSYSTEM_ASSESSMENT.md` and
`docs/REAL_HTTP_APPLICATION_PLAN.md`. ADR 0010 proposes local dependencies and
executable imports and is accepted. `project::packages::resolve` implements the
local graph only; executable package imports/linking remain unfinished.
`examples/library-foundations` is tested
inline composition, not cross-file/package consumption. Preserve these boundaries.

## Commands and verification

```text
cargo build
cargo fmt --all -- --check
cargo check
cargo test --no-run
cargo test -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- run examples/hello-world/main.svr
cargo run -- check examples/fielddesk
cargo run -- check --format json examples/functions/main.svr
cargo run -- build --emit js examples/hello-world/main.svr
```

CI runs formatting, strict Clippy, and `cargo test --all-targets` on Linux.
Install Node.js for backend execution tests (CI uses Node 22).
Windows CLI tests can return early on Application Control error 4551: inspect
output and report actual execution separately from compilation or skipped checks.
Do not weaken tests or bypass OS policy to obtain a green result.

## Change discipline

- Preserve compiler stage APIs and the Rust-owned trusted implementation.
- Use rustfmt (100-column setting), four-space indentation and LF. Unsafe code
  is forbidden; document public APIs and maintain Debug implementations.
- Reproduce bugs with focused regression tests before fixes. New language
  features need valid/invalid syntax, semantic and execution coverage as relevant.
- Syntax/semantics changes must synchronize specification, parser, AST,
  semantic analysis, IR/runtime/backends as applicable, tests, examples, docs,
  and course materials. Create `docs/course/` incrementally for verified features.
- Label proposals as planned. Preserve the bare `print` compatibility alias.
- Keep diagnostic codes stable; improve file identity/spans and structured
  output deliberately. Never equate scanner success with complete validation.
- Record significant work, validation, limitations and the next task in
  `docs/DEVELOPMENT_LOG.md`. Inspect dependencies/tests/design before replacing
  a subsystem and explain why incremental replacement is necessary.
- Continue authorized roadmap work through tested milestones. Document and
  pause before fundamental syntax, memory-model, type-semantics or compatibility
  decisions. Label features Implemented, Partial, Experimental, Stub or Planned;
  do not count scaffolding or skipped tests as completed functionality.
