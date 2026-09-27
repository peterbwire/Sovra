# Production-readiness audit and implementation plan

Audit date: 2026-09-27. **Not production-ready.** The user superseded the
developer-preview release plan and requested production-quality implementation.
This is a code-informed gap assessment, not a security certification or proof
that every defect has been found. Existing M0–M15 numbers remain unchanged;
“complete” foundation milestones describe slices, not release acceptance.

## Evidence and scope

Reviewed the public parser, semantic analyzer, IR, interpreter, JavaScript
emitter, project discovery/imports/application inspector, CLI command dispatch,
test harness and release workflows alongside accepted ADRs. Keep the existing
Rust-owned implementation and compiler stage APIs; no wholesale rewrite is
justified by this audit. Existing changes must remain intact.

The current baseline executes 192 tests on Windows. That is useful regression
evidence, not proof of full language/runtime correctness. Platform/MSRV workflows
are configured but have not been verified in hosted CI. No publication is
authorized by successful archive creation.

| Area | Evidence in code | Production gap / acceptance criterion |
| --- | --- | --- |
| Source compiler | `parser.rs`, `semantic.rs`: primitive types, straight-line functions and modules | Defined control flow and structured data; valid/invalid syntax and type tests, recovery and resource-limit tests. No accepted source may silently change meaning in lowering. |
| Types and IR | `TypedProgram` wraps a cloned AST; `ir.rs` is a public linear stack representation | Preserve resolved types/symbols into lowering, specify and validate public IR invariants, add branches only with approved semantics. Unknown types must remain errors, not placeholders that pass checks. |
| Runtime/backend | `interpreter.rs`, `backend.rs`, `numeric_runtime.js` | Equivalent observable results/errors for all supported operations. Float non-finite/formatting policy needs a decision. Add malformed-IR and boundary cases; scope allocation/output/depth limits explicitly. |
| Project checker (M12) | `project.rs` scans lines; `application.rs` parses a limited separate subset | A structured project representation must own declarations, imports, bodies and types. Unsupported syntax cannot imply validation. Migrate incrementally and preserve diagnostics/JSON contracts. |
| Services | Contract metadata and scoped call name/arity checks exist | Define implementation scopes, resolve signature types, check argument/return types, and validate implementations. Execute service tests before claiming runtime support. |
| Modules/libraries | Inline executable modules; project-local service imports only | Specify package identity, exports and dependency manifests; implement local dependencies, reproducible lockfiles, resolver/cache, then publishing/installing/version compatibility. Test two independent packages and clean-machine reproduction. |
| Sovra tests (M13) | `svr test` is reserved | Implement discovery, execution, failure locations and machine-readable results; tests must fail reliably and cannot silently skip required execution. |
| Application runtime (M14) | Source `run` only; Fielddesk cannot execute | Approved execution/lifecycle/concurrency contracts, implemented application services, failure propagation, cleanup and integration tests. Business concepts remain libraries, not core primitives. |
| Inspection (M15) | JSON diagnostics, contract metadata, member references | Link references to canonical package/symbol identities with accurate source locations and coverage; no “success” claims for unvalidated scope. |
| Reliability | Focused tests, depth bound, checked Int arithmetic | Malformed-input corpus, fuzz/property testing, parser/AST recursion limits, deterministic diagnostics, large-input tests and regression cases for every confirmed bug. |
| Release operations | Platform/MSRV jobs and archive scripts | Passing exact-commit builds on supported hosts; extracted-artifact smoke tests; selected version/repository, upgrade policy, reproducible installation and documented support/security reporting. |

## Implementation order and completion gates

1. **Existing-subset correctness (active).** Build a backend differential matrix
   covering primitives, conversions, output, calls, errors and malformed IR.
   Close confirmed discrepancies before extending syntax. Audit parser recursion,
   allocation/output growth and filesystem discovery. Approve Float/resource
   policies before enforcement that changes accepted behavior. Gate: targeted
   regressions, complete Windows suite, strict Clippy and verified platform/MSRV
   jobs; documented remaining limits must not be marketed as complete parity.
2. **Structured project checking (M12).** Introduce retained application nodes
   using the current inspector as a starting point; migrate declarations and
   signatures incrementally from scanning. Specify service implementation scope
   in an ADR before implementation. Resolve primitive and declared types and
   connect typed references to bodies. Gate: invalid signatures/bodies rejected,
   full supported-project coverage, cross-file visibility and diagnostic tests.
3. **Language foundations needed by applications.** Design control flow,
   structured data and ownership/memory semantics before implementing them through
   parser, analysis, IR and both execution engines. Gate: executable examples and
   positive/negative/differential tests for each accepted feature; no placeholder
   “type checked” values. No memory model is selected by this plan.
4. **Package lifecycle and Sovra testing (M13).** Approve package/import/export
   contracts, deliver local dependencies and locked resolution before a registry.
   Add package-aware native tests, then publication, installation and ownership
   rules. Gate: independently authored libraries publish/install under the chosen
   distribution design; lockfile reproducibility and failure cases pass.
5. **Application execution and inspection (M14/M15).** Deliver runtime behavior
   and typed machine-readable project boundaries on those foundations. Gate:
   supported end-to-end applications actually run, fail predictably and shut down
   cleanly; Fielddesk remains a proposal until its used features pass those gates.
6. **Production release qualification.** Run stability/regression/security review,
   platform acceptance, upgrade/install exercises and user documentation against
   the exact candidate revision. Gate: no unresolved correctness blockers in the
   advertised scope, all above acceptance evidence recorded, release ownership
   and version settled. Packaging-only checks are necessary but insufficient.

## First concrete correction

The audit found that generated JavaScript functions accepted the wrong number
of arguments from public IR, while the interpreter rejected them. A regression
reproduced this with a missing argument. The emitter now checks argument counts
before parameter setup and frame entry, matching interpreter errors. Tests cover
missing/excess arguments and an invalid entry-function signature. This closes
one invariant; verification passed 159 library and 34 CLI tests (193 total),
with strict CLI execution and no skips, plus formatting, strict Clippy and the
doc-test stage (zero doc tests). It does not establish full malformed-IR validation or production
readiness. Stack underflow and missing-name guards have since been added to
generated JavaScript with differential regressions. Runtime kind checks, builtin
arity, malformed literals and allocation bounds require continued audit. Builtin
arity and `std::len` runtime-type checks now match across engines. Interpreter
call argument counts are checked against the stack before allocation, eliminating
the reproduced capacity-overflow panic from an extreme malformed IR count.
Numeric IR literal parsing now follows Rust in both engines, with execution-time
errors for rejected spellings. This closes conversion discrepancies but does not
settle Float output policy or complete runtime-kind/resource validation.

## Decisions and progress discipline

Accepted ADRs remain authoritative. Fundamental syntax, type, memory and
compatibility choices require a concrete ADR and user review under AGENTS.md.
Do not infer those choices from “production level.” Continue implementation and
verification that does not depend on undecided rules; present design decisions
with examples and migration impact when they become the next dependency.

Record each completed gate with code/tests, exact validation results and remaining
limitations in DEVELOPMENT_LOG.md. Keep current Partial/Experimental labels until
the corresponding acceptance criteria are met. Do not erase limitations or change
labels to satisfy a release date. No deadline or final release version is assumed.
