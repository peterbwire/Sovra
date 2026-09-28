# Library ecosystem assessment

Audited 2026-09-28 against the working tree, including existing uncommitted M12
changes. The supplied 158-test baseline is historical. Before this work the
Windows suite executed 188 library and 36 CLI tests: **224 passed**, no skips,
with `SOVRA_REQUIRE_CLI_EXECUTION=1`. Evidence: `target/library-audit-baseline.log`.

## Actual capabilities

Follow-up after this audit: ADR 0010 was accepted. `project::packages::resolve`
now resolves local manifest graphs as a Rust API, with canonical identity,
cycle/duplicate/missing-path validation and containment. Executable package imports,
linking, CLI consumption and reproducible locks are still absent. The table below
records the original audit baseline; graph resolution has advanced to PARTIAL.

| Area | Status | Evidence and limit |
| --- | --- | --- |
| Inline language modules | IMPLEMENTED | `parser.rs`, `ast.rs`, `semantic.rs`: `mod name { export fn ... }` in one source; qualified calls use `name::function`. |
| Visibility and scope | IMPLEMENTED subset | Exported functions visible outside their module; private helpers callable by qualified name inside it (ADR 0005). Bare calls resolve top-level functions/builtins, not implicit siblings. Locals/parameters are per function. |
| Duplicates/collisions | IMPLEMENTED | E3008 rejects duplicate modules/functions, including private members. E3016 rejects exact builtin callable collisions, including bare `print`. Noncolliding `std` members remain allowed. No nested executable module hierarchy. |
| Standard library | IMPLEMENTED small registry | `stdlib.rs`: `std::print`, `std::println`, `std::len`, `std::to_string`, plus `print` alias. Typed signatures feed semantic checks; Rust interpreter and JS runtime implement behavior. `Any` is builtin parameter metadata, not a source type. |
| Executable source imports | PLANNED | Source parser accepts functions/inline modules; `run`/`build` read one file. Project imports do not link executable ASTs or IR. |
| Project imports | PARTIAL | `project/imports.rs` plus `validate_imports`: dotted project-relative file references, canonical containment, missing/ambiguous paths, direct service visibility. Module-reference cycles are allowed; this is not a package cycle policy. |
| Application body checks | EXPERIMENTAL | Opt-in structured service-call inspection; signature/arity checks are not application type checking. Accepted ADR 0009 is only partly implemented. |
| Manifest | PARTIAL | Custom parser, not general TOML: `[project]` name/version/entry, `[runtime]` target web/cli, `[services]` external bindings. Required name/entry, duplicate/unknown entries and malformed values are checked. Version text is not dependency selection. No dependency sections. |
| Types/data | IMPLEMENTED primitive subset | Unit, Bool, signed 64-bit Int, Float, String. Functions, local inference, explicit parameters, checked arithmetic. `Named` AST/type representation does not establish usable named types; executable unknown annotations fail E3017. |
| Collections/control flow | PLANNED | No runtime List/Map/Set/Tuple/Option/Result, record/enum values, loops, conditions, closures or general generic type system. Application annotation spellings do not implement these. |
| Package graph/lock/cache/publish | PLANNED | No resolver, lockfile, package loader, archive integrity or registry client. Rust Cargo dependencies are unrelated to Sovra dependencies. |
| Package CLI | STUB | Reserved install/update commands fail as unimplemented. No meaningful add/remove/search/info/publish pipeline exists. |
| HTTP and application execution | PLANNED | Route metadata is not a listening server. No socket, request parsing, body handling or network response path. Fielddesk is proposed application syntax. |

The requested TODO/FIXME/HACK/todo!/unimplemented!/placeholder/stub search in
source, tests and documentation found mostly documentation references; it did not
find a hidden package implementation. CLI dispatch confirms planned commands are
nonfunctional regardless of marker wording. This is a code audit, not a claim
that every defect has been found.

## Architectural boundaries

A **language module** owns source symbols and visibility. A **standard-library
module** is a module distributed with the toolchain; some operations require a
trusted host implementation. A **package** is a versioned distributable owner of
modules. A **dependency** is a declared edge from one package to another selected
package. An import selects module/symbol visibility; it must not select versions.

Keep package resolution, module loading, semantic resolution and execution
separate. Future identities must retain package + module + symbol instead of
flattening unrelated libraries into one string namespace. Canonical paths identify
local graph nodes, not immutable content or portable release identity.
See [package requirements](design/PACKAGES_AND_LIBRARIES.md) and the proposed
[local package decision](adr/0010-local-library-foundation.md).

## Standard-library and data dependencies

No registry refactor is justified merely to create empty folders. The current
four-function registry is small and already centralizes signatures. Preserve it
and the compatibility alias. Split implementations by responsibility when real
capabilities require it; keep interpreter/JS parity and one canonical signature
registry. Do not expose `std::strings` or other namespaces before they work.

| Capability | Useful foundation now | Missing prerequisite |
| --- | --- | --- |
| Strings | Concatenation, length, formatting wrappers | Index/slice policy, iteration for broader algorithms |
| Math | Typed arithmetic functions | Conditions for abs/min/clamp; domain/overflow contracts for new builtins |
| List/Array | None as values | Element types, indexing/bounds, ownership/mutation policy, iteration |
| Tuple | None | Product types, construction, access/destructuring |
| Map/Set | None | Collections plus equality/hash policy and deterministic iteration contract |
| Option/Result | None | Sum types, payloads, generic parameters, matching and propagation rules |
| I/O | Captured print output | Input/error values, host handles and backend capability contract |
| Testing | Rust harness tests Sovra | Assertions/failure semantics and Sovra test discovery/execution (M13) |
| Path/filesystem | No API | Path encoding/platform rules, errors, host authority and resource lifetime |
| JSON | Primitive scalar values only | Recursive structured values, collections, error locations, parsing/iteration |

Do not introduce arbitrary collection or ownership semantics to make sample code
appear executable. ADR 0009 approves nominal type direction, not generic, sum-type
or memory rules. A library implementation must not require compiler-specific
business keywords. Network/database/crypto/async work is deferred.

## Verified example and remaining gate

`examples/library-foundations/main.svr` composes exported string/arithmetic helpers
and private implementations using existing inline syntax. Its CLI regression
checks source validation, IR generation, interpreter execution and Node execution
of emitted JavaScript. It is **not** separate-package consumption or cross-file reuse.
Existing regressions cover private access, duplicate members/modules, builtin
collisions, std resolution, import containment and missing/ambiguous source files.
`std::len` measures UTF-8 bytes, not graphemes; the example's ASCII label does not
change that contract. Package missing/duplicate/cycle/security/lock tests cannot honestly be counted
until a package resolver exists.

Next milestone: approve executable import/local dependency rules, then implement
the smallest actual two-package check/run/build flow. Preserve M0–M15 numbering;
the requested local/graph/lock/Git/archive/registry/publish progression is a parallel
package work track. Completion requires that flow and negative resolution/privacy
tests, not just manifest records or these design files.

## Final validation for this slice

188 library + 37 CLI tests passed on Windows (**225 total**, no skips), including
the new real check/run/IR-build/JS-build/Node example regression. Formatting,
strict Clippy and the documentation-test stage also passed (zero doc tests).
Evidence: `target/library-foundations-tests.log`. No language feature, builtin,
package resolver or public package command was added; this is an audited
foundation and executable composition example, not the completed ecosystem.
