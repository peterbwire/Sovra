# Checked root-program call links

Status: **Partial G13 E1 compiler linking with opt-in Rust execution.**
`compiler::v2::check_program_source(label, source, ProgramSourceLimits)` checks
all root functions against one source-owned enum/record/alias environment and
signature table. Forward references, self-calls and mutual calls retain
`CallableTarget::ProgramFunction(ProgramFunctionId)` rather than being mistaken
for the current function. Every body/signature is checked, including unused
functions. Different nominal enum payload types remain incompatible; ordinary
checked numeric/array widening is retained at call boundaries.

This extends the [single-entry source projection](entry-source-v2.md) internally
without changing its public scope. The earlier `check_entry_source` still refuses
additional functions. [Matches inside arms](recursive-match-v2.md) are now supported
in program checking, including [structured if/while bodies](control-match-v2.md).
[Qualified same-file module functions](module-functions-v2.md) also support the
explicit v2 CLI and source-backed native host. Imports, nested modules, generics
and dependency linking remain unavailable. A program needs at least
one function, but this checker does not select a runtime entry. Existing main
parameter/return restrictions continue to apply. Root export intent is metadata.

`CheckedProgramSource` retains the original source/label, declaration ranges,
checked functions in source order and canonical `ProgramCallLink` edges. Standard
and constructor targets stay in each function's ordinary call evidence. Function
IDs carry a private request identity and ordinal: equal ordinals from independently
checked programs cannot be mixed. Source names are used during checking and explicit
inspection/entry lookup, not replaced with runtime string dispatch. Original
Unicode/CRLF byte/line/scalar-column coordinates survive the projections.

The immutable header table is shared across checked functions. Initial headers
are admitted before copying. Every per-body signature table, including all enum
constructors, consumes cumulative signature-record/text allowances before its
copies. Zero allowances refuse; there is no reset per body or unlimited fallback.
`signature_copies` and `signature_text_bytes` report actual charged totals.

`ProgramSourceLimits` also caps functions (default 128), retained canonical call
links and aggregate evidence items. Evidence counts include call/context facts,
evaluated expression facts, typed call/context links, declarations and value uses.
Each body's existing item caps are clamped to the remaining aggregate allowance;
the completed body batch is charged before retaining it in the public program.
Temporary checking can construct a batch that exceeds that remaining aggregate
and is then refused. Existing type-tree/text, function syntax and scope limits
remain **per function**, not aggregate program heap/work guarantees. Whole-source,
declaration and nominal-environment limits remain shared. Helper work, physical
allocation and Debug output are outside these logical quotas.

The mapper verifies program call heads against the same immutable header IDs.
Per-function expression/function IR can be emitted and validated with these
canonical targets. The separate [linked Rust runtime](program-execution-v2.md) now executes these
owned tables. The current standalone VM preflight-refuses any program
call, including unreachable calls, before instruction/value/service effects.
There is no fallback that executes a different function or resolves source names.
Functions without program calls may still use the existing standalone IR API.

Linked-program execution selects each callee's own graphs/scopes while sharing
invocation limits, control and services across frames. JavaScript/Worker/native formats and program/package/tooling
integration remain open under the [G13 contract](../design/G13_COMPLETION_CONTRACT.md).

The separate `parse_program_syntax` stage now supplies [syntax-only root
formatting](experimental-v2-format.md); parsing success is not checked-program success.

The program checker now supports [qualified same-file module functions](module-functions-v2.md),
including private/exported call filtering, scoped type contexts and canonical
callee signatures. The single-entry API stays strict; imports/dependency linking
and full root-type shadowing remain open.
