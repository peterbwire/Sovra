# ADR 0037: Source-unit AST admission and retained IR budgets

Status: **Accepted and implemented.** User approved the source-unit defaults.

## Implemented mechanisms

`resources::check_ast_budget(program, max_nodes)` counts borrowed AST nodes with
iterator frames, avoiding a cloned queue proportional to body/array width.
Modules, aliases, record declarations/fields, functions/parameters, statements
and expressions each count one. Program/collection containers and field-label
tuples do not add nodes. `SemanticAnalyzer::analyze_with_node_limit` applies the
admission check before normal semantic cloning. Exhaustion reports E3020 with
the encountered node's source range and returns no successful typed program.

This admission check receives an already parsed AST. It does not bound initial
parsing allocations, repeated semantic passes, alias resolution or imported
interface expansion. Structural depth checks remain separate.

`ir::lower_with_limits(typed, LoweringLimits)` bounds retained emitted instructions
and UTF-8 string payload bytes across all functions in that lowering invocation.
Strings include function/parameter names, literal text, load/store names, operator
text, call names, record identities/field names and field-access names. Repeated
owned occurrences count separately. Bool literals and numeric instruction fields
do not add string bytes. Function names count even when their bodies emit no IR.

The private emitter charges before appending each instruction and propagates
E3021/E3022 on exhaustion, discarding partial output. It exposes no mutable vector
access; branch patching changes only numeric jump targets. Future instruction
variants require an explicit string-accounting match. Function/parameter names
are charged before their IR copies are retained.

`ir::lower_program_with_limits(program, max_nodes, limits)` combines admission,
normal semantic validation and emission. Direct TypedProgram lowering remains a
trusted-stage API; manually constructing TypedProgram still bypasses semantics.
Stage API signatures remain unchanged. Resource errors use source context
from the function or the statement/expression being lowered, not a target-machine
instruction address, and never return a partial successful IR program.

## Approved source-unit defaults

- **250,000 AST nodes** admitted before semantic cloning.
- **1,000,000 retained IR instructions** per source-unit lowering.
- **64 MiB retained IR string payload bytes** per source-unit lowering.

These apply to standalone source units and equivalent checked unit APIs.
SemanticAnalyzer::analyze and semantic-only source check apply AST admission
without beginning to emit IR; ir::lower_program and standalone run/build/test
additionally enforce emission limits. Explicit budget APIs honor the caller's
selected allowance without a second hidden default check. Raw trusted ir::lower
continues returning IrProgram and has no implicit emission quota. Package
linking needs separate cumulative admission/emission accounting, including generated
qualified names and imported interfaces, before claiming an equivalent whole-graph
IR quota. The existing 64 MiB aggregate package entry-source limit remains.

## Measurement and limitations

Local measurements without executing programs:

| Source | AST nodes | IR instructions | IR string bytes |
| --- | ---: | ---: | ---: |
| examples/functions/main.svr | 19 | 13 | 72 |
| examples/records/main.svr | 40 | 25 | 137 |
| examples/scoped-types/main.svr | 71 | 48 | 258 |
| tests/fixtures/native-runtime.svr | 534 | 471 | 2,004 |

These small fixtures do not establish production-scale performance or a safe
process-memory/CPU guarantee. Profiles are retained in target/fuzz-evidence/.
Instruction/string ceilings bound retained IR payloads, not Vec capacity, allocator
overhead, helper maps, pending temporary payloads, backend output bytes or all
semantic operations. Source strings and an AST already exist before admission.
Empty programs contain zero counted nodes; an empty main still retains its name.

## Decision boundary

[AGENTS.md](../../AGENTS.md) requires: “Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions.” Default adoption
rejects some previously accepted valid source units. The concrete mechanisms
were implemented and tested before review; the user explicitly approved these
defaults. Package cumulative policy remains a separate decision.

## Verification

Tests cover exact/overflow/zero budgets, UTF-8, metadata and empty functions,
wide AST admission, ordinary semantic rejection, and every instruction cutoff
in a branching/loop/short-circuit/widening/record/array fixture. Successful bounded
lowering equals the original IR byte-for-byte in its structured representation;
errors do not corrupt branch patching or leak state into later invocations.
Source fuzzing also exercises the checked bounded path without executing programs.
