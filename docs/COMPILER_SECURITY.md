# Compiler security

Security is a continuing engineering requirement, not a claim that Sovra is the
most secure language. No independent security audit or comparative certification
has been completed. This document separates current safeguards from open work.

## Threat model

Treat source text, source-derived names and literals, local dependency contents,
and serialized IR as potentially hostile inputs. Protect the compiler process,
generated code integrity, diagnostic accuracy and existing output files.
Malformed input should fail with diagnostics instead of causing a panic, stack
overflow, unintended code execution or silent acceptance.

Checking and emitting IR/JavaScript must not execute Sovra source. Running source
or tests executes its file and process-I/O effects with the host user's authority;
these commands are not a sandbox. Native executable linking invokes a trusted
Rust/Cargo/linker toolchain. Cargo configuration, environment variables, executable
search paths and runtime source are part of that trusted build environment.
Payload hashes detect corruption; they do not authenticate a maliciously replaced
payload and matching manifest.

## Existing safeguards

- Rust implementation forbids unsafe code. Source type checking precedes normal
  lowering and execution; explicit public low-level APIs have separate contracts.
- Checked Int arithmetic and array bounds checks reject invalid operations.
  Interpreter and JavaScript behavior have differential regression tests.
- Source delimiter, expression and statement nesting is bounded before building
  excessive recursive trees. Caller-built ASTs receive a borrowed iterative depth
  preflight at semantic-analysis entry. `else` chains share the statement budget
  with nested blocks, even when few braces are open.
- Generated JavaScript escapes strings, parses numeric literals before rendering
  them and uses null-prototype maps for source-controlled function/field names.
- Native IR decoding bounds image size and counts, checks lengths and UTF-8,
  and rejects unknown instruction forms, invalid numeric literal payloads and
  invalid call/declaration metadata, including in unreachable instructions.
- Native output publication refuses overwrites; installed runtime payload checks
  reject missing/extra/modified files, links and excessive structural sizes.
- Default executable frontends enforce 16 MiB/source, 1,000,000 non-EOF tokens
  and 256 lexer/parser diagnostics, rejecting exhaustion without partial success.
  CLI source compilation, language tests and package entries use bounded reads
  of at most the byte ceiling plus one detection byte. The formatter retains
  its stricter 1 MiB read limit. Rust callers may select explicit FrontendLimits;
  see [ADR 0035](adr/0035-frontend-resource-budgets.md) for coverage and limitations.
- Executable package compilation defaults to 64 MiB total entry sources across
  canonical packages, including the root. Reads honor the remaining allowance
  before allocation; aliases/shared dependencies count once. Trusted Rust hosts
  may use `packages::compile_with_source_limit` for explicit aggregate policies.
- Default graph loading enforces 1 MiB per manifest, 8 MiB total canonical
  manifests and 8,192 alias edges. Initial project classification and ordinary
  project manifest reads also use the per-file cap. Trusted Rust hosts may
  select explicit PackageGraphLimits. See accepted
  [ADR 0036](adr/0036-package-graph-resource-budgets.md) for accounting and scope.
- Malformed-manifest parsing and executable semantic analysis retain at most
  256 errors, including a stopping notice (E4018/E3019). Production is bounded
  as errors are emitted, rather than trimming an already oversized report.
  Malformed classification aborts early, and exhausted semantic traversal skips
  later validation without returning successful partial output.
- Guarded compiler messages retain at most 4 KiB each, preserving UTF-8 and
  structured error metadata with a visible truncation suffix. Oversized backing
  allocations are replaced, not merely shortened. Semantic messages and migrated
  main project/manifest/package emitters format into a 4 KiB text buffer and stop
  standard Display/Debug
  producers on exhaustion, preserving the existing clipped text. Eager argument
  construction, other emitters' formatting, formatter internals, file identity,
  JSON escaping and raw caller-owned records remain outside this ceiling.
- Package cycle diagnostics stream borrowed paths rather than building an owned
  list/joined chain first. Project route/page path problems also borrow their
  source path and format directly into the final bounded diagnostic, preserving
  prefixes, Unicode clipping and validation behavior.
- CLI check I/O errors and language-test configuration/discovery errors use the
  bounded formatter before retaining messages. Full JSON path identity and
  source ranges are preserved; invalid test configurations stop before execution.
  Public application inspection/error-string APIs still have their existing raw
  contracts and require separate resource-policy work.
- Human error lines generated by the main CLI stream C0, DEL and C1 controls as
  printable uppercase Unicode escapes using the existing terminal-escape policy.
  This includes messages, source-file prefixes, command arguments and I/O paths,
  preventing those fields from injecting raw ANSI controls or extra error lines.
  Ordinary Unicode/backslashes, machine-readable decoded data, emitted code and
  intentional program output retain their existing behavior. Human escaping can
  expand retained message length; it is streamed rather than buffered in full.
- Formatter/doctor human diagnostic fields use the streaming terminal adapter
  while preserving static help/report layout. The shared check/test/doctor JSON
  string encoder emits DEL/C1 as standard JSON Unicode escapes alongside C0,
  preserving all decoded values and schema. This avoids raw controls in those
  serialized string fields; it is not an aggregate report-size or memory quota.
- Hosts can choose primary application-project discovery/read quotas through
  `check_project_with_scan_limits`. Counts bound scheduled directories, yielded
  entries and retained source paths; reads honor per-file/aggregate byte budgets
  and stop on failure. E4150 rejects exhaustion with no partial ProjectCheck.
  [ADR 0040](adr/0040-primary-project-scan-input-limits.md) defines approved
  defaults: 16 MiB/source, 64 MiB total, 4,096 source paths, 4,096 directories and
  100,000 yielded entries, enforced by ordinary primary checking and its CLI
  callers. Optional body/service inspection and
  aggregate errors/metadata are outside these input quotas.
- Rust hosts can opt into borrowed AST admission and retained IR instruction/
  string budgets. Emission is charged before retention and partial failures return
  no successful IR; [ADR 0037](adr/0037-source-unit-admission-and-ir-budgets.md)
  defines approved standalone defaults: 250,000 AST nodes, 1,000,000 retained IR
  instructions and 64 MiB retained IR string payload. Check enforces AST admission;
  standalone run/build/test additionally enforce emission quotas. Explicit callers
  can select limits; raw trusted lowering remains separate and package linking
  uses the graph policy below.
- Package hosts can select cumulative source AST, interface metadata and final
  linked IR quotas through `compile_with_compilation_limits`. The emitter charges
  qualified names before retention; interface imports copy signature metadata
  without cloning bodies. [ADR 0038](adr/0038-package-compilation-budgets.md)
  defines approved graph defaults: 1,000,000 AST nodes, 250,000 interface nodes,
  4,000,000 linked instructions and 256 MiB retained linked string payload.
  Package compiler APIs and executable check/run/build/test enforce them;
  interface string bytes, temporary storage and repeated semantic work are
  outside these quotas.
- Hosts can select cumulative interface string materialization through
  `compile_with_interface_string_limit`. Names, normalized annotations, record
  attachments, generated aliases and qualified signature labels are charged
  before their interface copies are retained. E3024 rejects exhaustion.
  [ADR 0039](adr/0039-interface-string-materialization.md) defines the approved
  128 MiB graph default, enforced by ordinary compiler APIs and executable
  package check/run/build/test. Explicit callers can select allowances.
  Semantic-stage AST copies, helper maps and type-resolution temporaries remain
  separate work.
- Package interface normalization reuses one immutable type registry per prepared
  canonical package, shared by export checks, record normalization and consumer
  imports. Registries live only for that compilation; consumer aliases and later
  compilations cannot change or reuse another owner's context. Every interface
  materialization still consumes its node/string quota. This removes repeated
  declaration-registry construction, not all repeated semantic operations.
- IR float-alias and record-field propagation uses a shared reverse-dependency
  worklist instead of repeatedly walking chains or rescanning all aliases.
  Record aliases share immutable float-field metadata, avoiding a copied field
  set per alias. Graph/helper storage remains outside complete compiler
  work/memory quotas.
- Lowering canonical-name lookup uses a per-invocation, iteratively resolved
  alias table instead of allocating a cycle set and walking a chain per lookup.
  Shared terminal strings avoid duplicating large identities across aliases.
  Defensive cycle behavior is preserved for trusted-stage inputs; checked source
  still rejects recursive aliases. Table construction/helper storage is not a
  complete process-memory quota.
- Inferred record facts share immutable identity strings with the lowering
  record registry. Copying facts between local bindings or
  through nested array metadata no longer duplicates the whole record name.
  Separate lowering invocations retain independent registries; runtime values
  and owned emitted IR names keep their existing behavior and accounting.

These are individual safeguards, not proof that every input or execution path is
safe. The source compiler does not impose a complete memory or CPU budget.

## Required verification

Every confirmed defect needs a failing regression before its fix. Run formatting,
strict Clippy and the full Rust suite. For structural changes also run
`node scripts/probe-compiler-depth.mjs --assert-limits` and
`node scripts/test-compiler-security.mjs`; both isolate compiler invocations with
timeouts. Preserve original source ranges and stable diagnostic codes. Successful
packaging or compilation alone does not establish security qualification.

`cargo test --test compiler_security` runs 4,096 deterministic source mutations
and 4,096 binary-image mutations, plus truncation/trailing-data checks. Successful
source mutations pass through analysis/lowering and IR round-tripping; rejected
ones retain valid diagnostic ranges. Mutated programs are never executed. CI
gives this regression target a two-minute process budget. These fixed-seed checks
are reproducible corpus tests, not sustained coverage-guided fuzzing.

Isolated coverage-guided source and native-image targets are available under
[fuzz/](../fuzz/README.md), with reviewed source seeds and generated image corpora.
They use libFuzzer and AddressSanitizer and never execute the generated programs.
Keep failure artifacts private until triaged. Bounded campaign results do not
establish exhaustive coverage; input limits are specific to the test harness.

## Open security work

1. Bound remaining valid-input semantic/IR work and transient diagnostic formatting.
   Per-file frontend limits do not establish an end-to-end memory or CPU quota;
   scanner-only project paths need separate resource coverage.
2. Run sustained coverage-guided campaigns against the source/native-image targets,
   expand seed diversity and input sizes, and retain minimized regression corpora.
   Short local campaigns and deterministic probes do not replace sustained fuzzing.
3. Audit native build configuration inheritance, dependency filesystem races and
   runtime-source provenance. Do not use native linking in an untrusted build
   environment as an isolation boundary.
4. Design execution isolation and resource/capability policy separately before
   offering untrusted-code hosting; current run/test commands provide neither.
5. Qualify supported hosts/MSRV, reproducible artifacts and dependency provenance,
   and commission independent review before making broad security claims.

Report vulnerabilities according to [the security policy](../SECURITY.md).

When extending executable semantic validation, use the scoped push_semantic
helper; manifest syntax errors use push_manifest_error. The public raw Diagnostics
container remains available and is not globally capped. Compiler entry-point
guarantees do not cover arbitrary caller mutations of that raw collection.

### Formatter handle checks (2026-11-12)

Formatter snapshot and conflict reads compare opened regular-file metadata before
reading. Linux x86/x86_64/aarch64 opens use nonblocking/no-follow flags to refuse
FIFO/link substitutions; Windows opens reparse points directly and rejects those
handles. Other architectures retain metadata admission without those Linux flags.
The FIFO/link/regular-file replacement regression is Linux-executed; new Windows
behavior remains an execution qualification gate. Parent-path changes and final
rename races still have the documented best-effort conflict semantics.

### Structured container exclusion (2026-11-12)

Auth-aware/app-aware syntax composition excludes only recognized containers at
supported original-token declaration boundaries. An envelope cannot interrupt a
service/function/type header and thereby repair invalid syntax after filtering.
Source tokens/coordinates remain original, EOF is retained, overlapping ranges
are rejected and unknown policy guards remain coverage gaps. This does not
authorize policies or establish full M12 validation.

### Render-header boundaries (2026-11-12)

The independent page/view header parser rejects a following supported declaration
before its body opener, preventing a missing body from consuming that declaration.
UI bodies and render/function-type result semantics remain explicitly unvalidated.
Original source spans and existing source/token/signature quotas remain active.


## Registered package source inventory (partial G12)

The opt-in [inventory API](reference/package-source-inventory.md) bounds explicit
module registrations, graph sources and observed sizes, checks portable paths and
canonical containment, and rejects regular-file identity aliases using Unix
metadata. Other hosts report unavailable identity checks. It neither opens source
content nor validates UTF-8/module headers/bodies. Metadata and paths may change;
this selection is not a verified snapshot or proof against concurrent filesystem
substitution. Compilation and future locks must validate bounded opened bytes
and consume those same bytes throughout checking/emission. Existing APIs reject
module manifests until integrated, preventing silently ignored registered inputs.


The additive [source-unit loader](reference/package-source-units.md) now performs
bounded capture and parsing on Linux x86/x86_64/aarch64 with no-follow/nonblocking
opens, opened-handle metadata and procfs location validation. Tests cover link,
FIFO, regular-file and parent-link substitutions. It retains captured buffers and
original AST ownership but does not yet type-check/link modules or retain raw
manifest snapshots for locks. Unsupported reader hosts fail explicitly; this is
not an atomic filesystem snapshot or a replacement for future locked integrity
verification of the exact manifests and sources consumed by compilation.


The additive [registered primitive-function compiler](reference/registered-package-functions.md)
now consumes captured units without source rereads, resolves direct file-local
callable imports, checks private/unused bodies and links only checked IR. Interface,
metadata and emission budgets are cumulative, including repeated import copies;
exhaustion never falls back to unchecked lowering. Record interfaces remain an
explicit unavailable context in this scoped API. Default CLI, lock verification
and broader host/tooling qualification are not introduced by this slice.


The separate [registered record compiler](reference/registered-package-records.md)
normalizes nominal primitive-field records without granting private source names
from attached canonical metadata. Provider field validation and caller errors keep
source ownership; public private-type leaks fail. Root-record/module-root internal
IDs are disambiguated. Record-valued fields now reference visible declarations
through canonical IDs without layout expansion. Public fields cannot leak private
types, and returned nested values do not grant transitive source aliases. All
interface/evidence/emission budgets still fail without unchecked fallback.


Executable CLI commands now classify module registration across declared package
graphs and use the complete checked registered pipeline, with no scanner/entry-only
fallback for invalid modules. Compilation precedes execution/emission, and package
testing retains root-only discovery. Unlocked classification/compilation use bounded
graph reads without claiming atomic snapshots or lock enforcement. Explicit
[source locks](reference/package-source-locks.md) verify the retained manifest/source
bytes consumed by compilation. Context/overlay paths remain strict; package
formatting now includes registered inventory sources without rewriting locks. See the [CLI contract](reference/registered-package-cli.md).


## Structural array annotation admission

The approved first G13 contract admits `[T]` annotations through bounded iterative
parsing and alias expansion. At most 127 array wrappers plus one leaf fit the
128-level type bound. Cycles or excess expansion fail before checked lowering.
Written wrappers count against AST admission; copied normalized wrappers count
against interface node/string allowances, including each imported signature.
Nominal element identity and exported-private leaf restrictions remain enforced.
Existing checked type/context/emission quotas remain active with no fallback.

`std::array_len` semantically accepts arrays only; interpreter and generated JS
also check runtime arguments, and existing builtin arity checks apply to raw IR.
Recursive numeric conversion preserves source snapshots across parameter,
initialization, return and record-field boundaries. Older partial application type
inspection refuses array declarations until its own privacy/identity integration.

These component limits do not bound total runtime allocation or dynamically built
compound values. [Iterative runtime value traversal](reference/runtime-values.md)
now hardens Rust cloning, runtime-owned cleanup and both engines' display, including
recursive record/array shapes. Explicit `Value::drop_iterative` supports caller-owned
Rust values without changing public enum destructuring; ordinary external Rust
Drop/Debug/PartialEq retain their earlier contracts. Typed source depth and these
work lists do not establish a runtime memory/CPU sandbox. [Optional execution
limits](reference/execution-limits.md) now meter dispatched IR instructions and
printed UTF-8 bytes, including LF, before the next instruction/host write. Counters
are shared across callees; bounded formatting and iterative exhaustion cleanup
preserve earlier host effects without fallback. Existing APIs/CLI/native policy
stay unchanged. These counters do not meter heap, helper work, files or blocked
host I/O. Allocation, full-work/interruption policy, general collections/errors,
binary and JSON APIs remain G13 work.


## Optional logical materialization fuel

The [approved model v1](design/G13_MATERIALIZATION_FUEL.md) now has additive
[Rust and JavaScript APIs](reference/materialization-limits.md). Independent u64
node/text allowances charge creation, snapshots and formatted payloads without
release refunds. Both dimensions admit atomically; host results have minimum-shape
preflight and complete admission before VM publication. Earlier host effects can
remain on failure. The read-only footprint observer enables no quota itself.
Logical fuel excludes physical heap/RSS, helper scratch, host memory and blocked
I/O. Existing CLI/native/default policies remain unchanged. This is a component
control for trusted hosts, not process isolation or a complete G13 implementation.


## Cooperative Rust execution control

The approved [Rust embedding control](reference/execution-control.md) adds optional
shared cancellation and monotonic elapsed checks. Initial validation keeps its
errors; checkpoints precede dispatch/fuel and surround host operations. Host Err
values retain priority, while successful callbacks are checked before VM ingress.
Completed effects and committed fuel remain on failure; cleanup stays iterative.
This cannot preempt helper work or blocked I/O and supplies no fixed stop latency,
process sandbox, new CLI/native policy or JavaScript cancellation guarantee.


## Explicit Node Worker cooperative control

The approved [worker generator](reference/javascript-worker-control.md) requires
Node Worker mode and validates a version-1 shared atomic flag when enabled.
Exact BigInt elapsed checks and lexical checkpoints preserve cancellation/host/
fuel ordering. Ordinary generators, CLI/native policies and defaults remain.
Workers share process file descriptors and filesystem authority; stdout proxy
options do not redirect direct fd-1 writes. No process sandbox, fixed cancellation
latency, helper/I/O preemption or browser/mobile transport is established.


## Snapshot array growth

[Append/concat](reference/array-growth.md) retain first-input element/nominal types
and only existing second-input numeric widening. Checked result length refusal
precedes atomic logical fuel admission and iterative snapshot construction.
Input containers are not charged as output children; all copied descendants and
labels are counted once. Runtime raw-IR container checks do not reconstruct static
element types. The representation ceiling is not an allocator/RSS limit, and
helper work remains non-preemptible at the approved cooperative boundaries.
