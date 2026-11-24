# ADR 0041: Retain checked type and symbol evidence

Status: **Implemented; G08 portable behavior and Linux qualification complete
(2026-11-12).** Immutable checked ownership, types/uses/contexts, bounded IDE
projection and ordinary standalone/package lowering consumers are implemented.
Active metadata admission limits and E3025 are documented in the
[policy](../reference/checked-evidence-policy.md). Syntax, numeric conversions,
legacy public stage APIs and default check JSON schema remain compatible;
exceeding new metadata quotas deliberately adds compilation rejection.
Opt-in JSON additions are documented in the [report contract](../reference/check-json.md).
This is retained compiler evidence, not a new type system or optimizing typed IR.

The sections below record incremental implementation history. Later activation
and completion notes supersede earlier pending/default-proposal statements.
Windows/macOS/MSRV and clean-machine release qualification remain separate gates;
G08 completion is not security certification or whole-language maturity.

## G08 completion evidence

- All 609 ordinary Rust tests pass, including 4,096 deterministic source mutations,
  checked/legacy IR and diagnostic parity, quotas, imported identities and normal
  caller migration. One compile-fail doctest and strict all-target Clippy pass.
- Final compiler/runtime corpus: 21 observations over five cases; checked-evidence
  corpus: all 16 cases preserve exact legacy IR with three samples per mode.
  The 13-process aggregate corpus verifies active/default compilation boundaries.
  Observed overhead is recorded, not a universal resource bound or speed claim.
- Final source ASan/libFuzzer campaign also exercises the active compiler helper
  and trivia parity without executing inputs. It reuses the working corpus;
  short campaigns do not replace sustained fuzzing or independent review.
- Current compiler passes strict symbol regressions, relocated installed tooling
  with source-verified VSIX/full LSP, and real VS Code 1.106 Linux host activation,
  inferred hover/shadowing, edits, restart and check/IR/JS/run tasks with spaced paths.
  The extension is local and unpublished.
- Local source/binary/log hashes and scope are recorded in
  `target/checked-evidence/g08-final-verification.json`. Raw local artifacts are
  not source-controlled release attestations. Next contributor task is G09,
  package-aware LSP/unsaved-workspace navigation; Windows G04/G05 remain deferred.

Legacy public lowering APIs and explicit package legacy lowering intentionally
remain for compatibility and reference comparisons. Ordinary command consumers
use complete checked metadata; no exhaustion falls back to legacy inference.

## Implemented foundation

[checked.rs](../../src/compiler/checked.rs) exposes `CheckedUnit::from_source`
and `CheckedUnit::from_program` with mandatory caller-selected `EvidenceLimits`.
The snapshot privately owns the normalized checked AST and structural index;
clones share immutable ownership. Source construction parses/checks the exact
text retained; AST-only construction returns no original source. No arbitrary
AST-plus-source constructor is exposed. Existing `TypedProgram` APIs are intact.

`NodeId` is a snapshot-local ordinal; `NodeRecord` carries its kind, span and
parent. The record itself does not identify a resolved binding or semantic type;
optional type facts refer to its NodeId.
The index follows deterministic category preorder (aliases, records, functions,
modules), with AST-vector child order. As in existing AST admission, Program and
collection containers are excluded; expression statements and their expression
each count. Equal or synthetic spans remain separate structural nodes.

The caller's node allowance is checked by borrowed counting before semantic
cloning/index allocation. Source allowance is checked before parsing/retention.
The normalized index also charges every record before insertion. Existing default
AST/frontend/depth checks remain independent; indexing recurses only after depth
admission. `CheckError` separates node/source exhaustion from unchanged ordinary
diagnostics, and no partial snapshot is returned. These allowances bound node
count and retained source bytes, not Vec capacity, all AST payloads or process RSS.
Type and flow storage allowances are described below; no ordinary CLI defaults
have been added.

[Snapshot tests](../../tests/checked_snapshot.rs) cover all current structural
forms, deterministic parent/ordinal ordering, exact/overflow/zero node/source
allowances, Unicode byte spans, immutable shared ownership, detached legacy AST
mutation, equal synthetic spans and existing parse/semantic/depth failures. A
compile-fail doctest rejects mutation through the borrowed AST accessor.
Opt-in CLI/IDE projection now uses source-backed snapshots (slice 3 below).
Normal package/lowering callers remain pending; an explicit standalone checked
lowering API is implemented below. Construction from a caller AST
deliberately makes no source-token navigation claim.

## Implemented type recording (first part of slice 2)

`CheckedUnit::from_source_with_evidence` and `from_program_with_evidence` add
explicit `SemanticEvidenceLimits`. Existing structural-only constructors and
their `CheckError` variants remain unchanged. The new `EvidenceError` wraps those
failures or reports semantic recording exhaustion. Collection returns no partial
snapshot. If checking fails, original diagnostics take precedence even when
recording has exhausted its allowance; checking is not stopped or rewritten by
the recorder.

The existing semantic checker now optionally records value-expression results,
parameter types and local binding types where it establishes them. Legacy paths
pass a disabled recorder with no evidence/index allocation. Initializers and
bindings are recorded separately; existing scope clones and resolution determine
their types. There is no second type-inference traversal. A structural lookup
temporarily translates borrowed normalized AST addresses to deterministic NodeIds
during checking. These addresses are never serialized, used as identities or
retained in the completed snapshot; aliases are finalized without changing AST
structure after checking.

`SemanticEvidence` owns a type table and `TypeFact` records keyed by structural
NodeId. Scalars, canonical standalone nominal names and array layers are interned;
arrays refer to existing child TypeIds. Alias spelling remains in the AST/source.
Unknown inference holes remain explicit, including nested empty arrays, and do
not become Any. Borrowed array traversal avoids repeated nested-type clones.
Facts are in checking order, not structural-ID order. TypeIds and NodeIds remain
snapshot-local. Borrowed tables are immutable and shared by snapshot clones.

The recorder charges `max_facts`, `max_types` (including array layers/Unknown)
and `max_name_bytes` (unique nominal UTF-8 names) before retaining records/types/
name payload. Intern lookup keys share name storage. These are retained-item/
payload quotas, excluding Vec/map capacity, temporary canonicalization work and
all existing semantic allocations. No production defaults or diagnostic codes
are selected. Complete evidence for these supported categories is returned or
the whole recording request fails; no truncated type snapshot is exposed.

Function-call heads are not first-class value types. Some assignment targets are
validated directly by the checker rather than through value-expression checking.
They therefore have no type fact in this stage; absence is distinct from an
explicit Unknown type. Flow-enabled construction adds resolved uses separately.
Inferred function types and package provenance remain planned; bounded JSON/LSP
local type display is implemented in slice 3 below.

[Type recording tests](../../tests/checked_types.rs) cover aliases/module scope,
shadowed locals, initializer-vs-binding widening types, record fields, builtin
results, nested holes, interning, exact/overflow/zero quotas, equal spans,
diagnostic precedence and unchanged normalized AST/IR. The ordinary 4,096-source
mutation campaign additionally compares recorded/legacy ASTs and full diagnostics.
The flow-enabled extension below finishes the current standalone slice 2 scope.

## Implemented resolved uses and conversion edges (rest of slice 2)

`from_source_with_flow_evidence` and `from_program_with_flow_evidence` additionally
require `FlowEvidenceLimits`. Their semantic evidence exposes `flow(): Some(...)`;
type-only constructors still expose None and keep their existing API/limits.
Declarations, uses and coercions are immutable and snapshot-local. Opt-in CLI/IDE
projection uses this API. The explicit standalone checked lowering API below also
uses these facts; normal package/lowering callers remain pending.

The checker records source function, parameter and local declarations. Declaration
identity is its structural NodeId, independent of name spelling or span. Function
names retain their unqualified spelling; the structural parent supplies module
context. Value scopes follow existing function/branch/loop boundaries: initializers
resolve before the new binding is installed and branch/loop exit restores outer
declarations. Direct/indexed writes and receiver reads have distinct use kinds.
Calls use the exact function object selected by the existing function namespace,
even when a same-spelled value binding exists. Builtins point to the canonical
Rust registry name (including bare-print compatibility), without invented source
locations. No name resolution is repeated from source text.

Conversions preserve the expression's intrinsic type and record a separate
contextual edge: initialization, assignment, indexed assignment, return, argument,
record field, array element, or mixed numeric binary operand. Each edge identifies
its owner/value nodes, from/to TypeIds and array depth. Only Int-to-Float leaves
with matching array nesting produce edges. Identity conversions, Unknown inference
holes and nonnumeric values do not produce invented conversions. Array item edges
use already recorded checker results and the final inferred element type, not a
second inference walk. Mixed arithmetic/comparison edges describe numeric casting
already performed by the runtimes; they are not instructions to add redundant
IR widening around Binary. Backend placement remains a later migration concern.

Flow allowances bound declaration count, use count, coercion count and retained
declaration-name UTF-8 bytes. Names are charged per declaration before allocation;
scope tables share those strings. Builtin names refer to static registry storage.
Types introduced by contextual edges share the explicit semantic type/name limits.
Index/source limits remain independent. Helper maps are bounded by admitted
node/fact/declaration counts but their capacity is not a byte quota. Exhaustion
discards the whole snapshot, while invalid source retains original diagnostics.
No production defaults, process-wide memory/CPU limits or runtime sandbox implied.

[Flow tests](../../tests/checked_flow.rs) cover initializer shadowing, restoration
across then/else/loop scopes, function/value namespace separation, canonical builtin
calls, reads/writes, module functions with equal synthetic spans, all eight
conversion contexts, nested array depths, holes/identity omissions, exact/zero/
overflow limits and diagnostic precedence. Leading Unicode trivia preserves IDs,
types, uses and conversions while shifting byte ranges. The ordinary 4,096-source
mutation campaign compares legacy, type-only and flow-enabled outcomes.

This finishes standalone function/value evidence, not all future symbol kinds.
Alias/record/field type navigation, annotation-token links, source name-token
projection, package-qualified provenance and unsaved workspace imports remain
outside these APIs' current claims. IDE projection is implemented below; checked
lowering and package consumers are slice 4. G08 is not complete until its consumers and
qualification gates pass.

## Implemented CLI/IDE projection (slice 3)

`check --format json --symbols` now constructs a source-backed checked snapshot
and projects its declaration/use identities and local types. Removed independent
scope/name resolution and AST call-target reconstruction from IDE collection.
Lexer tokens are used only for source ranges, annotation spelling, module display
and bounded signature excerpts. Invalid source emits its original diagnostics
without evidence; default JSON/check/build/run behavior remains unchanged.

Unannotated locals add nullable `inferred_type` and show concrete scalar, nominal
or `Array<T>` labels only from retained facts. Explicit annotations preserve source
spelling. An inference hole at any array depth, or an oversized type label, gives
unavailable display rather than a guessed/partial type. Included references map
only to emitted declaration indices; omitted shadow declarations never redirect
to an outer binding. Functions, value reads/writes and builtin calls all use
recorded targets; field/type navigation remains outside this slice.

The opt-in request uses existing 250,000-node admission for source/index/fact/
declaration/use/coercion counts, 8,192 interned types and 512 KiB each for nominal
and declaration-name payload. Source byte admission remains the ordinary frontend
allowance. Projection retains the existing 8,192-entry table and 512 KiB output
budgets, with 4 KiB signature/type labels. These are IDE-request profiles, not new
language acceptance defaults. On collection exhaustion, retry ordinary checking:
valid source returns empty arrays with `unavailable: true, truncated: true`;
ordinary source errors still fail. No incomplete snapshot is projected. Unit
tests inject zero quotas, and actual CLI/LSP tests exercise amplified qualified
names and overlong metadata. This is not a process-wide memory/CPU guarantee.

The LSP decoder validates envelope/table/string limits, inferred metadata,
definition indices and exact UTF-8 boundaries before converting ranges to UTF-16.
Malformed evidence fails operationally and never becomes type/definition data.
Missing metadata and explicit unavailable state remain distinct from empty-but-
complete evidence. Legacy symbol reports without `inferred_type` remain readable.
Array label depth is bounded by its 4 KiB display budget, not syntax depth: inferred
nesting can grow through successive shallow bindings. A 129-level regression
reproduced and corrected the initial decoder's overly strict syntax-depth check.

Compiler, LSP, relocated tools and VS Code tests cover inferred scalars/records/
nested arrays, holes, source annotations, scope identities, UTF-16/CRLF, metadata
truncation/unavailability and edits/failure/close invalidation. VS Code continues
to render escaped plaintext Markdown, without trusting compiler text as commands
or links. The strict decoder is included in the developer-tools allowlist/VSIX.
No browser-editor LSP, package roots, new semantic completion or type navigation
is implied. The standalone checked-lowering API is implemented below; package/
normal caller migration and final overhead/parity qualification remain pending.

## Standalone checked lowering (first part of slice 4)

`CheckedUnit::from_source_for_lowering` / `from_program_for_lowering` add explicit
`LoweringEvidenceLimits`. They retain canonical declared/default function results
and contextual expected types, including identity edges. Existing non-identity
coercion records alone cannot preserve exact old IR: Float parameters, returns,
bindings and record fields may emit identity WidenFloat instructions. Context
collection therefore preserves that information without changing numeric rules.
Function results also include implicit Unit, charged against the existing explicit
semantic type allowance. Old structural/type-only/flow APIs do not collect these
records and retain their original limits and behavior.

The new [ir::lower_checked](../../src/compiler/ir.rs) requires a lowering-ready
snapshot and caller-selected emission budgets. Missing metadata returns
`CheckedLoweringError::EvidenceUnavailable`; no rechecking or partial type
reconstruction occurs. Collection quotas return no partial snapshot; emission
failure returns existing E3021/E3022 diagnostics with no partial IR. Original
source diagnostics take precedence over exhausted metadata allowances.

[checked_facts.rs](../../src/compiler/ir/checked_facts.rs) adapts retained
expression/binding/result types, resolved function/value targets, canonical record
identities and field contexts to the shared emitter. The checked entry point does
not build the legacy alias, return or field-inference tables. Persistent IDs stay
structural; temporary borrowed-address lookup is rebuilt against the immutable
snapshot. Variable names retain the old deterministic shadow-name policy, but
reads/writes resolve by stored declaration identity. Function and builtin calls
use recorded targets, preserving bare-print IR spelling, including one-part
qualified paths from caller ASTs. No new source syntax is implied by that case.

Per-emission mutable binding-name state belongs to the adapter, not the snapshot.
Snapshot clones remain immutable/Send/Sync and repeated emissions start fresh.
Private lowering array facts now share immutable Arc-backed children instead of
repeatedly cloning owned nested type trees. New contexts/results have independent
explicit record allowances and share semantic type/name budgets. Temporary
adapter maps/capacity remain outside IR payload and total process-memory quotas.

[Checked lowering tests](../../tests/checked_lowering.rs) compare exact IR and
interpreter results for all eight runnable standalone examples plus focused
numeric/record/array/alias/scope/control-flow programs. They compare every smaller
instruction/string allowance to legacy diagnostics, cover exact/zero context
limits and diagnostic precedence, and reject type-only/flow-only snapshots.
Equal synthetic spans and the caller-AST print-alias regression preserve identity
and spelling compatibility. The 4,096 ordinary source mutations now also compare
lowering-ready analysis and exact checked/legacy IR for accepted programs;
mutated programs are never executed.

## Import-aware checked package compilation (2026-11-12)

The opt-in `project::packages::compile_with_checked_evidence` accepts existing
compilation limits plus explicit `PackageEvidenceLimits`, returning linked IR,
canonical unit count and remaining metadata allowances. Ordinary CLI, test,
native and package entry points still use legacy lowering. G08 remains partial:
qualify retained-evidence overhead and production defaults before promoting them.

Resolver-only snapshots check the exact normalized imported signatures selected
by the existing checker. Separate import and imported-call tables retain graph
owner ordinals, import IDs and canonical linked targets; `UseTarget` keeps its
existing local/builtin variants. Foreign functions and parameters never become
fabricated local declarations. Transitive record identities retain their original
provider, even through re-exports and multiple aliases. Graph ordinals are scoped
to one compilation, not persistent identities or content locks. These augmented
AST snapshots have no owned source: foreign interface spans do not imply local
source navigation. Package-aware IDE projection is not implemented by this API.

Metadata allowances are cumulative across canonical units, charging retained
consumer interface copies, import spellings/targets and local plus imported uses.
Distinct aliases charge distinct bindings; canonical providers compile once.
Source AST limits and attached-interface evidence limits measure different data.
Existing graph, source, interface-string and shared IR emission limits remain in
force. Temporary helper capacities are not total memory/CPU quotas. Exhaustion
returns a separate evidence error with the unit entry path, without fabricating a
source diagnostic range. During admitted semantic checking, real semantic errors
precede collector exhaustion; early metadata admission or later interface
validation does not provide a universal diagnostic-precedence guarantee.

Tests compare exact legacy/checked linked IR, interpreter and JavaScript results,
relocated graphs, refreshed providers, multiple aliases, private exports and
transitive nominal identities. Every smaller retained-metadata allowance and
linked instruction/string budget is checked. The full Rust suite passes 602 tests.
Keep the legacy path until normal caller migration and qualification pass.

## Initial evidence-overhead observation (2026-11-12)

The versioned [checked-evidence corpus](../../benchmarks/checked-evidence/README.md)
now compares release legacy/checked compilation in fresh processes, with exact
IR parity on every warmup/sample and retained counters beside wall time/peak RSS.
Nine cases cover the existing runtime corpus, larger context tables, a local
package and alias-heavy interfaces. The largest source case shows substantial
local overhead (about 65 ms/35 MB checked versus 39 ms/23 MB legacy). These include
process/output overhead, not isolated compiler-stage or allocator measurements;
small-case memory is masked by the launch floor. This is initial Linux evidence,
not completed overhead qualification or a production-limit decision. The alias
case confirms import entries require independent cumulative limits even with only
two canonical units. Preserve explicit APIs and legacy defaults while additional
shape qualification, production policy and normal caller migration remain pending.

## Shape qualification and candidate policy (2026-11-12)

Corpus version 2 preserves exact IR in all 14 cases with five samples per mode,
adding deep inferred types, long names, nominal types, wide calls and shared
package dependencies. A focused regression verifies exact and one-below quotas,
explicit evidence exhaustion and clean retries across those independent dimensions.
The [production policy proposal](../reference/checked-evidence-policy.md) records
finite standalone/graph candidates, lifetime/accounting rules and the required
exhaustion contract. These candidates are not active defaults or completed
resource qualification. Next test profile saturation and map metadata failures to
stable normal-command diagnostics before caller migration. G08 remains partial.

## Candidate boundaries and diagnostic conversion (2026-11-12)

Additive error conversions now map metadata exhaustion to E3025/null location,
preserving existing diagnostics byte-for-byte. Package entry provenance remains
available on the structured error before conversion; no fabricated source range
is attached. Existing normal commands and IDE fallback policy are unchanged.
Regression coverage verifies real evidence exhaustion, semantic-error precedence,
JSON failure/null location and unchanged source diagnostics.

The manual release saturation probe accepts exactly 250,000 structural nodes and
65,536 interned types in separate shapes, and rejects each next reachable shape
with the expected typed error. Initial whole-process RSS is roughly 105/109 MB,
including parsing/checking but excluding lowering. These are not memory bounds.
See the [policy observations](../reference/checked-evidence-policy.md) for commands
and remaining combined-table/graph/lowering qualification. Candidates remain
proposed; these four boundary cases do not justify default promotion.

## Combined lowering/package peaks (2026-11-12)

Corpus version 3 exercises the proposed profiles in all 16 cases, preserving exact
legacy IR for every warmup and three samples per mode. Combined nominal/context
metadata peaks around 136 MB versus 112 MB legacy; a five-unit/32,768-function
provider graph peaks around 206 MB versus 188 MB. Exact-node and exact-type
boundary probes now include lowering (about 132 MB and 109 MB respectively).
These process observations include transient work and do not bound compiler memory
or saturate all graph/table/IR limits. The [policy](../reference/checked-evidence-policy.md)
records scope and the remaining aggregate-boundary/default decision. No ordinary
caller, default budget, language semantics or IDE fallback changed in this slice.

## Aggregate graph/actual-IR boundaries (2026-11-12)

The manual five-unit graph corpus reaches exactly 1,000,000 canonical-source AST
nodes. Exact linked IR parity holds for 999,994 instructions/500,078 string bytes;
exact caller-selected output allowances pass and one-below allowances preserve
E3021/E3022 diagnostics. The next AST node preserves E3020. A cumulative evidence
allowance one below the graph's node count produces E3025 although every unit fits.
Local successful peak RSS is roughly 408 MB checked versus 366 MB legacy, including
output serialization. This does not saturate every metadata/default IR table or
establish a hard memory bound. No ordinary compiler defaults changed.

The [policy observations](../reference/checked-evidence-policy.md) and benchmark
README record scope, commands and remaining limitations. Next decide active limits
and their compatibility implications, then migrate standalone callers before
package callers. Retain exact source/IR/runtime parity and separate IDE fallback.

## Standalone executable caller activation (2026-11-12)

`compiler::compile_program` now centralizes existing AST admission, bounded
AST-only evidence collection and checked lowering with unchanged IR limits.
CLI run/build (including native output) and standalone language tests use it.
The standalone [policy profile](../reference/checked-evidence-policy.md) is active;
package defaults/callers and plain semantic check remain unchanged. Legacy public
stage APIs remain available. The compatibility impact is explicit: previously valid
programs exceeding new metadata ceilings can fail executable compilation as E3025,
while plain semantic check can still accept them. No syntax/type/runtime changes
or silent fallback are introduced. Language-test discovery uses normalized checked
syntax, and metadata JSON retains null locations.

Regression coverage compares ordinary-helper IR/errors, preserves AST E3020,
exercises actual active type-budget exhaustion and verifies language-test JSON.
Package migration and final G08 qualification remain next.

## Ordinary package caller activation (2026-11-12)

Ordinary package compilation now supplies `PackageEvidenceLimits::DEFAULT` to
checked graph compilation, preserving canonical imported type/call ownership and
shared source/interface/IR counters. All public ordinary source/graph/interface/
compilation-limit variants migrate together, including package CLI checking and
language-test compilation. Explicit legacy lowering remains only as a public
reference/compatibility API; no quota failure triggers fallback.

The [active graph policy](../reference/checked-evidence-policy.md) documents new
E3025 admission failures for previously valid metadata-heavy graphs. Updated
boundary corpus version 2 exercises the ordinary default path, identical exact
IR/located diagnostics, and a four-provider nominal-type graph that legacy accepts
but the active cumulative type limit rejects. Existing return types and standalone
semantic check/IDE fallback remain compatible. Final G08 qualification remains.

## Problem established before implementation (historical motivation)

- [semantic.rs](../../src/compiler/semantic.rs) computes expression/binding types
  and resolves aliases while checking. `TypedProgram` retains a public `program`
  field containing a validated, normalized AST, without per-expression facts.
  Callers can construct or mutate that wrapper. It is a trusted-stage input, not
  an immutable proof object.
- [ir/type_facts.rs](../../src/compiler/ir/type_facts.rs) reconstructs partial
  Float/record/array facts for widening. Its `Other` category is intentionally
  less expressive than semantic types. Lowering also maintains lexical bindings.
- [ide_symbols.rs](../../src/compiler/ide_symbols.rs) and its local collector
  revisit checked syntax and lexer tokens for declaration/reference identity.
  Inferred local type hover is explicitly unavailable. The CLI checks source
  successfully, discards the semantic result, then renders from the original AST.
- Package compilation supplies normalized imported signatures separately. Source
  offsets alone cannot identify declarations across different source units.

These independent reconstructions make new IDE features harder to keep aligned
with checking and coercion behavior. Do not infer that an existing divergence or
security vulnerability has been demonstrated; this is an architectural gap.

## Recommended representation

Introduce an additive `CheckedUnit` with private fields and borrowed read-only
accessors. It owns the normalized AST and its complete semantic evidence from
one successful analysis. Only the semantic analyzer constructs it. Cloning a
checked unit preserves the same immutable snapshot; exposing an AST clone does
not let a caller mutate the checked snapshot. Do not expose mutable accessors or
a constructor accepting arbitrary AST/fact pairs.

Keep `TypedProgram`, `analyze`, and existing lowering entry points compatible
during migration. Do not attach trusted facts to its mutable public AST. Add an
explicit checked-analysis entry point with caller-supplied evidence limits, then
a separate checked-lowering entry point. Legacy trusted-stage lowering retains
its documented contract until a separately reviewed API migration. Never use
facts copied from another `TypedProgram` or silently revalidate using different
imports to repair a mismatched snapshot.

Suggested logical records (names are provisional, not public API promises):

| Record | Identity and contents |
| --- | --- |
| Unit identity | Snapshot-local source-unit ID, source identity and import environment identity. Standalone checks use one unit; package compilation supplies canonical resolved package identities. |
| Node identity | Dense ordinal plus node kind within one unit, assigned by a deterministic walk. Unique even when caller-built nodes have equal/zero spans. |
| Declaration identity | Unit + declaration ordinal, namespace/kind, name and declaration/name-token byte ranges; optional original annotation range. |
| Resolved type | Interned scalar, array element, or nominal record identity; preserve alias spelling separately for display. |
| Expression fact | Node ID, checked result type or explicit unresolved inference hole, and resolved declaration/call target where available. |
| Binding fact | Declaration ID, checked binding type, mutability, parameter/local kind, and initializer Node ID. |
| Use fact | Node ID, resolved Declaration ID or registry builtin identity; value and function namespaces stay distinct. |
| Coercion fact | Use/assignment/argument/return/field/array edge, source and target types, existing widening operation and nested array depth. |

IDs are stable only within a snapshot. Do not persist raw indices as cross-edit
identities. An LSP document generation owns its snapshot; URI/version reuse after
close cannot reuse old IDs. A source digest may detect accidental mismatch but
is not authorization or proof that an import environment matches.

Spans remain original UTF-8 byte ranges. Store name-token ranges during source
indexing, without changing the semantic meaning of normalized annotations.
LSP converts ranges to UTF-16 only at its boundary. A caller-built AST lacking
original source tokens may support lowering facts but must report source
navigation ranges unavailable rather than guessing token positions.
For source-backed navigation, one constructor must parse/index/check its owned
source snapshot (shared immutable storage is acceptable). Do not accept an
arbitrary AST plus unrelated caller-provided text and pretend their token ranges
are associated. Charge retained source storage explicitly; sharing a source
buffer avoids another full copy but does not make its memory cost disappear.

## Collection rules

1. Check existing depth/node admission before cloning or indexing. Index the
   normalized AST with deterministic structural IDs. AST normalization must
   preserve structure or deliberately remap IDs; pointer addresses and byte spans
   are not node identities. The checker and consumers use the same indexed view,
   including nodes visited repeatedly by checking.
2. Record each fact where the existing checker establishes it. Do not add a
   second inference walk. Separate an expression's intrinsic type from the type
   expected at an assignment/call/return edge. Record coercions at those edges
   using current compatibility/widening rules.
3. Resolve an initializer before installing its new local declaration. Preserve
   current scope restoration across branches/loops, mutability rules and
   independent function/value namespaces. Do not turn facts into new flow-sensitive
   typing, captures, definite assignment or changed shadowing behavior.
4. Nominal records resolve to declaration identities, including imported records.
   Preserve current aliases for display while storing their checked canonical
   meaning. Two consumer aliases for the same imported record share its identity;
   different packages' same-spelled records remain distinct. Reuse resolver and
   interface normalization results, never a fresh filesystem search.
5. Preserve existing inference holes (for example empty array elements) explicitly.
   `Unknown`, a missing fact and a concrete type are distinct. Do not display a
   guessed type, synthesize Any or grant an unresolved value new permissions.
6. On semantic failure publish existing diagnostics and no successful checked
   snapshot. Discard partial semantic facts. Optional recovery-aware IDE analysis
   is separate future work, not part of this proposal.

## Resource and failure contract

Evidence is not an unbounded cache. Intern names/types once per checked snapshot;
bound node/declaration/use records, interned type nodes and retained UTF-8 payload
separately, including imported metadata. Check charges before allocation/copy and
use checked arithmetic. Avoid repeated owned copies of deeply nested array types
and long qualified names. Release the snapshot on replacement/close.

First implement an explicit `EvidenceLimits` API and a distinct evidence-collection
failure, without selecting new ordinary CLI defaults or diagnostic codes in this
ADR. Profile exact-limit/overflow/zero cases before proposing defaults. Existing
frontend, AST, graph/interface and IR budgets remain in force independently.
Do not describe any of these counters as total process-memory or CPU quotas.

A checked artifact cannot have silently incomplete mandatory facts: collection
exhaustion returns no artifact, and checked lowering cannot fall back to partial
inference. The caller may separately request an ordinary diagnostic-only check
under the unchanged legacy contract. Optional IDE export may be bounded/truncated
only at its projection boundary, with explicit unavailable/truncated state and
valid references into the emitted declaration tables. This is distinct from a
complete compiler evidence snapshot. Existing JSON's default shape stays intact.

## Consumer migration and compatibility gates

| Slice | Deliverable and required verification |
| --- | --- |
| 1: immutable/indexed snapshot | Private checked ownership, deterministic unit/node IDs and caller-supplied limits. Existing public construction/use of `TypedProgram` still compiles; modifying a detached AST cannot affect the snapshot. Equal spans do not alias IDs. |
| 2: record semantic evidence | Record parameter/local/expression types and resolved uses during existing checks, then aliases/records/builtin calls and coercion edges. Existing success/error codes, ranges and checking outcomes match exactly. Invalid source yields no artifact. |
| 3: IDE projection | Replace the independent local binding-resolution walk with checked declarations/uses. Add inferred local hover from concrete facts only; preserve annotation spelling and unavailable holes. Default check JSON stays unchanged; opt-in additions get documented fixtures and strict bounded LSP decoding. |
| 4: checked lowering | Consume retained result/coercion/target facts; remove reconstructed `TypeFacts` only after all in-repo callers, including package compilation, migrate. Compare exact IR and interpreter/JS outputs on existing widening, alias, array, record and import tests before removing the old path. |
| 5: qualify and finish G08 | Run normal compiler/resource/package tests, LSP/installed tooling, property/fuzz checks and the versioned performance corpus. Record retained evidence overhead and review proposed production defaults separately. |

Slices are incremental implementation boundaries, not permission to advertise
full typed IR or mark consumer migration done after adding containers. A mismatch
between new facts and current behavior must be investigated with a regression;
do not change established language semantics merely to make the migration pass.

Required focused cases:

- `let x: Int = 1; if true { let x = x + 1; print(x); } print(x);`:
  initializer use resolves outer x; inner print resolves new x; final print
  resolves outer x. Same-spelled functions/locals remain in separate namespaces.
- `let x: Float = 1`: initializer fact Int, binding fact Float, conversion attached
  to the initialization edge. Cover subsequent assignments, parameter/return
  widening and nested arrays/record fields with current parity fixtures.
- Aliases of scalars/records, module-qualified names, imported records via two
  aliases, and different packages' same-spelled records preserve current identity.
- Empty/nested empty arrays, invalid annotations, unresolved calls and failed
  checks expose no fabricated concrete type. Success with an inference hole is
  distinguishable from evidence exhaustion and semantic failure.
- Unicode/CRLF token ranges, duplicate/equal synthetic spans, exact/overflow/zero
  budgets, imported metadata accounting and deep types do not alias or escape limits.
- Edit/cancel/close/reopen discards old snapshots, including reset versions;
  rejected or failed buffers cannot expose the previous snapshot's type hover.

## Alternatives and boundaries

Appending facts to public `TypedProgram.program` risks stale metadata; replacing
that public wrapper in place breaks construction/access compatibility. Keeping
three independent resolvers retains drift. Using a full new typed AST/HIR now
would widen the migration without first establishing bounded shared facts.
The additive snapshot is the recommended first step.

This work keeps portable Rust semantic evidence independent of Windows, Node,
LSP and UI protocols. It does not implement package-aware unsaved workspaces
(G09), a new formatter, application scanner/M12 unification, generics, new type
rules, mobile/browser hosts, runtime isolation or Task Manager features.
