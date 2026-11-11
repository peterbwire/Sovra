# ADR 0038: Cumulative package compilation budgets

Status: **Accepted and implemented.** User approved the graph defaults.

## Implemented

`packages::compile_with_compilation_limits(root, PackageCompilationLimits)` adds
cumulative syntax, interface and final linked IR accounting. Existing graph
resolution/manifest/source defaults still apply. Ordinary package APIs and their
CLI callers enforce the approved compilation ceilings below; the explicit API
honors caller-selected compilation quotas without a second hidden default check.

- `max_ast_nodes`: the ADR 0037 node definition, summed over unique canonical
  package entry ASTs, including root. Checked after each parse, before retaining
  that unit and before semantic cloning. Aliases to one canonical dependency do
  not count its source twice. Exhaustion reports E3020 in the excess unit.
- `max_interface_nodes`: cumulative metadata materializations. A normalized
  function signature and each parameter count one; this includes exported
  signature validation and every import occurrence, even repeated imports.
  Exported-record cache entries and fields, consumer record attachments and
  generated imported-type aliases count one each. Attached records deduplicate
  by canonical identity within a consumer; aliases deduplicate by spelling.
  Nodes are charged before the corresponding metadata clone/normalization.
  Exhaustion reports E3023. Consumer-side copies of imported records use the
  import statement's location, preserving source-file/range provenance.
- `lowering`: one shared instruction/string budget across all packages. Function
  names, call targets and record identities are rewritten during emission, before
  charging and retention. Final qualified names count at their linked UTF-8
  lengths. Names/parameters of empty functions count; numeric jump patching does
  not add payload. E3021/E3022 returns no successful partial linked program.

Signature normalization now copies metadata directly instead of cloning a full
function body and clearing it for each import. Shared lowering still uses the
consumer's normalized imported types for widening and inference. No Sovra package
code executes during compilation.

## Approved graph defaults

- **1,000,000 cumulative source AST nodes**.
- **250,000 cumulative interface metadata nodes**.
- **4,000,000 linked IR instructions**.
- **256 MiB retained linked IR string payload**.

These apply to executable package compilation, including package check,
run/build/test paths using that compiler, not the partial application scanner.
They are graph limits, independent of the standalone source-unit ceilings.
The existing 64 MiB canonical entry-source total, per-file frontend ceilings,
manifest/edge budgets and 1,024-package cap remain separate.

## Evidence and boundaries

Local profiles without program execution:

| Graph | Packages | AST nodes | Interface nodes | IR instructions | IR strings |
| --- | ---: | ---: | ---: | ---: | ---: |
| examples/local-packages/app | 2 | 63 | 18 | 45 | 409 bytes |
| examples/local-packages/utilities | 1 | 33 | 7 | 21 | 190 bytes |

These small examples do not establish production-scale performance or safe
process-memory/CPU bounds. Profiles are retained in target/fuzz-evidence/.
Tests independently sum final linked payloads and reject every smaller instruction
and string allowance. They cover exact/overflow/zero budgets, two aliases to one
canonical dependency, record attachment/cache copies, import-source ranges,
ordinary semantic errors, empty entry names and successful execution parity.
The existing full suite exercises package float/record/private-function behavior.

ASTs are already parsed when admitted. Interface nodes do not bound interface
string bytes, repeated type resolution or internal temporary signature objects.
Source/prepared/typed AST copies, helper maps, allocator overhead, temporary
payloads, backend serialization and runtime execution remain outside the IR
payload ceiling. These are component quotas, not a compiler-wide memory budget.

## Decision boundary

[AGENTS.md](../../AGENTS.md) requires: “Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions.” Enabling the
approved defaults rejects some previously accepted valid graphs. The opt-in
implementation was complete and tested before review; the user explicitly
approved default activation. compile, compile_with_source_limit and
compile_with_graph_limits all retain the approved compilation defaults. Package
test compilation uses the same default root compiler as package check/run/build.
