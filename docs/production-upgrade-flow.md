# Production Upgrade Flow

This is the handoff flow for continuing the production-grade Sovra upgrade.
Use it when picking up M12 and later project/runtime work.

The current release target is production readiness, not a developer preview.
Follow [PRODUCTION_READINESS.md](PRODUCTION_READINESS.md) for audited gaps,
dependency order and acceptance gates. Keep verification-artifact infrastructure;
do not treat successful packaging as release approval.

## Current Spine

1. Harden `svr check <project>` before `svr test` or project runtime work.
2. Keep the CLI thin; production rules should live behind compiler/project APIs.
3. Validate the Fielddesk app surface incrementally: manifest, source discovery,
   services, routes, pages, auth, data models, scheduled tasks, service
   contracts, policies, then richer page/model checks.
4. Treat each checker rule as a public contract: stable diagnostic code, focused
   test coverage, and matching docs.
5. Keep shallow scanners conservative until the parser owns application syntax.
   A malformed declaration should produce a diagnostic, not disappear silently.

## Current Checkpoint

Primitive service signature resolution is implemented behind
`project::service_types::resolve_service_signatures` and opt-in
`check --service-calls`. It retains canonical owners and resolved types; unknown
annotations produce E4117 and owner failures E4118. Literal argument checking now
reports E4119 and permits Int-to-Float widening. Primitive parameters and inferred
locals now propagate through lexical scopes. Primitive annotated initializers are
checked with E4120 and widening; only validated types propagate. Primitive binary
arithmetic/comparisons now propagate types and report incompatible operands with
E4121. Valid resolved service calls now propagate contract return types, including
direct imports. Known explicit service returns now validate against contracts with
E4122. E4123 now checks non-Unit return paths through if/else and while bodies,
conservatively treating loops as potentially skipped. E4124 rejects known non-Bool
conditions. E4125/E4126 now prevent unresolved conditions/service returns from
passing silently. Same-file ordinary calls now resolve primitive interfaces before
body inspection, validate arguments and propagate result types. Call
checking also resolves the Rust-owned stdlib registry, including bare print,
without granting wildcard semantics to user Any annotations. Ordinary return
contracts and duplicate declarations are checked. Next, resolve cross-file ordinary
calls, application named types and remaining unknown expressions.
Unresolved ordinary calls now fail explicitly with E4133. JSON ordinary-call
records expose resolution kind, primitive type evidence and source ranges to
support inspection while these remaining boundaries are implemented.
Ordinary project checks remain partial wiring checks.

M12 now validates manifest metadata, source discovery, service bindings, app
routes, page routes, auth target wiring, app data model references, and
scheduled task targets. This pass also validates auth policy shape and policy
model references. The handoff assessment identified prerequisite executable
subset correctness gaps. All module function bodies now receive semantic
checks. Numeric conversion/bounds, checked Int arithmetic, JS numeric kinds,
token ranges and branch-aware return completeness have since been hardened.
ADR 0002 is approved and implemented: executable function parameters require
explicit types (`E3014`), with local inference and the default Unit return type
preserved. Expression diagnostics now retain precise source ranges. Named-type
resolution and richer diagnostics remain
before broader type-system claims. The project scanner does not enforce source
semantic rules. See `FULL_DEVELOPMENT_STATUS.md` before further service-contract
and model/page expansion.

## Verification Flow

Agents can request `svr check --format json <path>` for structured outcomes.
See `reference/check-json.md`: project success remains a wiring check, and
manifest parsing, source scanning and declaration-based value/wiring errors
retain file/line ranges. Missing keys, discovery and I/O errors keep null locations.
Malformed entry service/data lists now report E4024/E4062 at the declaration
instead of silently losing invalid items; list parsing remains single-line.
Source scanning now strips `//` comments separately from manifest `#` comments,
preserving markers within quoted strings and original diagnostic line ranges.

Run these in order after each slice:

```text
cargo fmt --check
cargo check
cargo test --no-run
cargo test
```

On this machine, Windows Application Control may block executing freshly built
Rust test binaries or `target/debug/svr.exe`. If that happens, record the block
and rely on `cargo test --no-run` plus compile checks until policy allows local
execution.

## Next M12 Slices

1. Service contracts: structured headers/parameters/return text and JSON operation
   metadata are implemented with passing Windows regressions. Service-call
   references use accepted ADR 0006 import scopes and ADR 0008 explicit service
   implementation scopes. Opt-in service-call checks validate operation names
   and arity in supported bodies; signature type resolution remains unfinished.
2. Page bindings: validate page targets, view references, and data dependencies
   once the application parser can provide structured nodes.
