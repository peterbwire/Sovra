# Checked-evidence production policy

Status: **Standalone and package profiles active, 2026-11-12.**
G08 portable behavior/Linux qualification is complete; release-platform gates
remain separate. Standalone run/build (including native emission) and language
test compilation now use retained evidence through `compiler::compile_program`.
Public low-level checked constructors still require explicit limits. Ordinary
package compilation/check/run/build/test now uses cumulative retained evidence.
Plain standalone semantic check remains on its existing path. This policy concerns portable compiler metadata,
independent of UI/host platforms and execution budgets.

## Candidate qualification profiles

These finite candidates are starting points for boundary and shape testing,
not a promise that every source accepted by today's compiler fits. The numbers
are not derived by scaling one benchmark's maximum. Promotion must explicitly
review newly rejected valid programs and stable diagnostics first.

| Retained quantity | Active standalone | Active whole package graph |
| --- | ---: | ---: |
| Structural nodes | 250,000 | 1,250,000 |
| Type facts | 250,000 | 1,250,000 |
| Interned types, counting array layers/holes | 65,536 | 262,144 |
| Unique nominal UTF-8 bytes, per snapshot copies charged | 16 MiB | 64 MiB |
| Declarations | 250,000 | 1,250,000 |
| Local plus imported uses | 250,000 | 1,250,000 |
| Nonidentity coercions | 250,000 | 1,250,000 |
| Declaration-name UTF-8 bytes | 16 MiB | 64 MiB |
| Function result records | 250,000 | 1,250,000 |
| Contextual edges, including identity | 1,000,000 | 5,000,000 |
| Imported bindings | Not applicable | 250,000 |
| Import spelling plus canonical-target UTF-8 bytes | Not applicable | 64 MiB |
| Owned source copy for ordinary AST-only lowering | 0 | 0 |

Structural candidates follow existing standalone AST and aggregate source plus
interface-node ceilings. Other record ceilings are independent finite allowances,
not proven ratios to syntax size. Types get a separate lower ceiling because
many distinct inferred array layers can occur in shallow syntax. Context ceilings
allow several edges per source expression without assuming one edge per node.
Payload limits are independent of counts: long names and repeated alias bindings
can consume bytes without many source nodes. Graph quotas count canonical providers
once but consumer interface/import copies each time. MiB means 1,048,576 bytes.

These are payload/record ceilings, **not total compiler memory limits**. Vec/hash
capacity, transient checker and resolver tables, AST clones, recursive checking,
IR and output serialization require additional memory. Before promotion, measure
candidate saturation on realistic deployment memory budgets; lower or redesign
limits if the retained footprint is unacceptable. Increasing limits must receive
its own evidence and compatibility review. No unlimited retry is permitted.

## Required behavior on migration

- Keep source, tokens, AST admission, graph, interface nodes/strings and shared IR
  limits in force. Evidence does not replace or reset any of them.
- Ordinary lowering should use AST-only snapshots. Source-owning editor snapshots
  need their own explicit source lifetime/budget and invalidation policy.
- Charge every table independently before retained growth. Type interning charges
  unique entries within one snapshot; graph aggregation charges per-unit copies.
  Local and imported uses share one allowance. Distinct aliases charge distinct
  import spellings/targets, even when the provider is canonicalized once.
- Freeze only successful complete snapshots. On exhaustion, publish no partial
  metadata or IR. Drop each package snapshot after its IR is emitted; do not retain
  the entire graph's source/metadata merely to return linked IR.
- Report explicit metadata exhaustion rather than silently falling back to legacy
  inference, raising limits or resetting a graph quota. Preserve current semantic
  errors during admitted checking; do not promise precedence over earlier AST
  admission or later interface validation. The additive `EvidenceError::into_diagnostics` and
  `CheckedPackageError::into_diagnostics` now implement E3025 with a null location
  for metadata exhaustion, preserving original source diagnostics. Structured
  package errors retain the canonical entry until conversion; the diagnostic
  conversion deliberately does not invent an entry-file range. Normal commands
  have not migrated to these conversions.
- Preserve canonical imported type/call ownership and exact legacy IR/runtime
  behavior. Migration must exercise CLI, test, native and package callers; checked
  IDE symbols have a separate bounded projection contract.

## Qualification evidence and remaining gates

[Corpus version 2](../../benchmarks/checked-evidence/README.md) adds inferred
array depth, long binding names, many nominal identities, wide call coercions and
a ten-unit diamond graph. Five samples per mode preserve exact IR in all 14 cases.
The 192-layer inferred type has only 579 syntax nodes but 194 interned types;
256 long binding names charge 258,454 bytes; 64 wide calls charge 4,096 coercions.
These show why parser depth, source node counts and declaration counts do not
substitute for independent evidence quotas.

`tests/checked_evidence_shapes.rs` tests exact allowances, every positive table's
one-below allowance, typed exhaustion, and fresh exact-budget retries with exact
legacy IR on deep types, long names and nominal/coercion pressure. Existing checked
package tests cover independent cumulative quotas and imported interfaces.

Initial standalone saturation and additive diagnostic mapping are implemented.
Still required: combined graph saturation and resource-sensitive shapes beyond
this corpus, an explicit decision on active limits, normal caller
migration, source/IR/runtime parity and platform/toolchain qualification. Local
Linux timings and passing quotas do not certify security or Windows/mobile support.

## Initial candidate saturation: Linux, 2026-11-12

The manual release `checked_evidence_saturation` example constructs inputs and
asserts candidate-profile acceptance/exhaustion without executing programs:

```sh
cargo build --locked --release --example checked_evidence_saturation
python3 scripts/benchmark-process.py target/release/examples/checked_evidence_saturation nodes exact
python3 scripts/benchmark-process.py target/release/examples/checked_evidence_saturation nodes overflow
python3 scripts/benchmark-process.py target/release/examples/checked_evidence_saturation types exact
python3 scripts/benchmark-process.py target/release/examples/checked_evidence_saturation types overflow
```

Use the `.exe` suffix on Windows; no Windows execution is claimed. Each example
process asserts the expected typed error before mapping it to E3025. Node overflow
is the next reachable shape (250,002 nodes); interned-type overflow is 65,537 types.
These are manual resource observations rather than slow mandatory CI tests.

A local single sample accepted exactly 250,000 nodes at about 105 MB peak process
RSS, and exactly 65,536 interned types (131,072 nodes) at about 109 MB. Both overflow
shapes were rejected as expected. The retained local artifact is
`target/benchmarks/checked-evidence-saturation.json`. These include source generation,
parsing, retained snapshots and transient checker tables, but **exclude lowering**.
They are not peak-memory bounds or deployment acceptance. Individual table maxima
are not simultaneous worst cases; package graph candidates remain unqualified.

This is evidence against treating the record ceilings as cheap or inferring
production memory use from tiny programs. Keep candidates proposed. Next qualify
graph/table combinations and lowering-time peaks, then decide whether to reduce
or restructure candidate profiles before normal caller migration.

## Combined lowering and package observation: Linux, 2026-11-12

[Corpus version 3](../../benchmarks/checked-evidence/README.md) runs all 16 cases
under the proposed candidate profiles with exact legacy IR parity on every
warmup and three measured samples per mode. A combined record/factory case charges
147,464 nodes, 16,387 types and 49,153 contexts at roughly 136 MB process RSS
(legacy roughly 112 MB). A five-unit graph with four 8,192-function providers
charges 327,729 structural nodes and 131,080 contexts at roughly 206 MB (legacy
188 MB). These include lowering and output; they do not saturate aggregate graph
ceilings. Package snapshots are per-unit; existing prepared AST/interface caches
and accumulated linked IR also contribute to memory use.

The manual exact-node boundary now also lowers successfully at roughly 132 MB
process RSS, emitting 249,998 instructions. The exact-type boundary lowers at
roughly 109 MB but emits zero instructions, so it cannot qualify worst-case
combined type/IR retention. See the corpus for commands and artifact paths.

Decision: retain candidate profiles without raising them and keep normal defaults
unchanged in this slice. Measurements establish successful bounded examples,
not an acceptable universal footprint. Before migration, decide active profiles
with their compatibility impact, and qualify a larger aggregate graph/IR boundary
rather than extrapolating these RSS values. Platform-specific qualification remains
separate; no mobile/Windows memory-support claim follows from Linux observations.

## Aggregate graph/IR boundaries: Linux, 2026-11-12

The [aggregate boundary corpus](../../benchmarks/checked-evidence/README.md)
now reaches the existing 1,000,000-node canonical-source graph ceiling across
five units. Checked and legacy IR agree exactly: 999,994 instructions and
500,078 owned string bytes. Exact caller-selected linked IR limits succeed;
one-below limits preserve identical located E3021/E3022 diagnostics, and one
extra AST node preserves E3020. A checked-only 999,999-node metadata allowance
rejects cumulatively with E3025 although each individual unit fits.

Initial successful whole-process peak RSS is about 408 MB checked versus 366 MB
legacy. These include lowering and debug IR serialization and are single samples,
not memory ceilings or portable timing results. The graph does not simultaneously
saturate interface copies, nominal types, contextual edges and default IR limits.
All independent existing compiler quotas remain active. No candidate was raised.

This closes the planned aggregate source/actual-IR boundary experiment. Next make
the active-limit/compatibility decision and migrate normal callers incrementally,
starting with standalone compilation and exact diagnostics/runtime parity. Keep
package migration separate so cumulative evidence and shared emission accounting
remain reviewable. Candidate profiles are still proposed in normal APIs until
that decision is implemented; G08 remains partial.

## Standalone activation decision: 2026-11-12

Adopt the standalone column above for executable run/build/test compilation,
without raising any candidate value or changing source/IR quotas. The shared
`compiler::compile_program` checks existing AST admission first (preserving E3020),
checks once with bounded evidence, then lowers exclusively from that evidence.
It returns normalized immutable syntax for language-test discovery and retains
no source copy. Missing lowering metadata is an internal constructor invariant;
there is no fallback inference or unlimited retry. Public legacy stage APIs stay
available for compatibility and parity comparisons.

Compatibility impact: previously semantically valid programs exceeding a selected
metadata table can now fail run/build/test with E3025. In particular, 65,536 distinct
nominal parameter types plus Unit exceed the 65,536 interned-type ceiling. Plain
semantic `check` can still accept such a program; successful checking does not
promise that later compilation/resource admission will succeed. Syntax, type
semantics, accepted in-budget IR and execution are unchanged. Real semantic errors
still precede collector exhaustion during admitted checking. Language-test JSON
keeps E3025 locations null rather than inventing a range from the input path.

The observed metadata cost is accepted for this bounded compiler migration,
without claiming any universal memory bound or target qualification. Package
profile activation/migration remains separate because graph-wide accounting and
interface copies require their own caller review. IDE projection keeps its smaller
limits and unavailable-evidence behavior; it does not inherit these executable
compilation defaults.

## Package activation decision: 2026-11-12

Activate the graph column as `PackageEvidenceLimits::DEFAULT` without raising
any candidate. All ordinary package compilation entry points, including source,
graph, interface-string and compilation-limit variants, collect import-aware
checked evidence and lower through the existing shared IR budget. CLI package
check/run/build/native output and package language tests inherit this path.
The root syntax returned for discovery comes from the normalized checked unit;
only linked IR and existing prepared AST/interface caches survive unit emission.

Compatibility impact: semantically valid graphs exceeding cumulative metadata
ceilings can now fail ordinary compilation with E3025, including package check
because it compiles executable dependencies. Four providers each with 65,536
nominal parameter types plus Unit exceed the graph's 262,144 interned-type cap,
even though each unit fits and the graph remains within AST admission. A manual
boundary corpus verifies legacy acceptance and active default rejection. Earlier
source/manifest/AST/interface/IR diagnostics remain unchanged when those limits
are encountered first; admitted semantic errors still precede collector exhaustion.

`compile_with_legacy_lowering` is an explicit public compatibility/reference
entry point with existing source/graph/interface and caller-selected IR limits.
It is used by parity probes/tests and is never called as an exhaustion fallback.
Low-level `compile_with_checked_evidence` still accepts explicit metadata limits
and returns structured exhaustion/provenance. Public ordinary return types remain
compatible, with metadata failures converted to E3025/null location. Input and
IR limits remain independent, cumulative and shared across canonical units.

Updated aggregate corpus version 2 verifies default checked versus explicit legacy
IR/diagnostic parity, cumulative node exhaustion and active cumulative type
rejection. G08 now awaits final qualification/documentation gates rather than
ordinary caller migration; no security or target certification is implied.
