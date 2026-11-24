# Opt-in Rust execution of versioned function IR

Status: **Partial E1 Rust execution.** `FunctionIr::execute` runs the sealed
[single-function IR](function-ir-v2.md) with [model-v2 values](runtime-values-v2.md).
This library API does not change CLI parsing/default execution, package/tooling,
JavaScript/Worker or native-image support.

The initial adapter executes literals, checked binding snapshots, conversions,
arithmetic/comparisons, arrays/records/member/index reads and indexed stores,
scopes, branches/loops, enum constructors, once-captured exhaustive matches,
guarded payload snapshots and returns. Self-recursion uses iterative VM frames;
return/error cleanup drops flat owned values without recursive compound cleanup.
Argument type/nominal/schema validation precedes entry snapshots. Each callee
copies its arguments under the same invocation budget; return values transfer.

Supported standard calls are exactly `std::print` (including `print`),
`std::println`, `std::to_string`, `std::len` (UTF-8 byte count),
`std::split_once`, `std::parse_int`, `std::lines_unique`, `std::escape_terminal`,
`std::array_len`, `std::array_slice`, `std::array_append` and `std::array_concat`. All other
standard calls, including unreachable calls, refuse in preflight before source
execution. There is no legacy fallback or general RuntimeHost adapter. Print
captures memory-only lines; no process stdout, input, filesystem, environment,
network or subprocess effects are published. Captured lines are returned only on
success. Extending the adapter requires its actual cost/effect and host contracts.

`FunctionIr::execute_with_arguments(entry_arguments, program_arguments, policy)`
adds `std::arg_count`, `std::arg` and `std::set_exit_code` using an explicitly
borrowed `&[String]`. Entry values remain separate from program argument text.
It returns `RuntimeVmCommandOutcome { execution, exit_code }`; status starts at
zero on every invocation and is shared by self-calls. The VM never reads actual
process arguments or changes/exits the process. Status/output are returned only
on success; callers decide whether to publish them. Ordinary `execute` still
refuses these calls in preflight, even in unreachable code.

Argument count costs one Int node. Argument selection validates the signed index,
checks cooperative control around observation, then admits one String plus all
selected UTF-8 bytes before copying. Caller-owned argument storage is outside
logical fuel until read; repeated reads charge fresh snapshots. Exit status must
be 0..=125. Validation precedes Unit admission, which precedes updating captured
status. Invalid indices/statuses refuse before result fuel. All frames share the
same invocation budget/context; none of these operations grants input/file or
other ambient host authority. Input/file calls still refuse in these two APIs.

`FunctionIr::execute_with_input(entry_arguments, program_arguments, input, policy)`
additionally enables `std::read_line` through an explicitly supplied `RuntimeVmInput`
trait object. This narrow interface exposes only line input. Print remains captured
in memory; there is no automatic process input/output or filesystem access.
File calls still refuse during preflight, including unreachable calls, before any
input is read. The earlier execute/execute_with_arguments APIs retain their refusals.

The host returns `Option<String>` without a line terminator: None means EOF,
Some("") means an empty non-EOF line. The standard `std::InputLine` record retains
its eof/text fields. The VM preflights three nodes plus all record/field labels
without consuming fuel before reading, checks cooperative control around the
host call, then admits the entire returned text before constructing the result.
Host errors produce no value and consume no result fuel. Hosts should bound
input while reading; the VM rejects successful returns over the existing
1,048,576 UTF-8 byte limit before result admission. Host allocation is outside
logical fuel. Input may already be consumed when cancellation, size or returned-
text admission refuses; there is no rollback. Blocking reads are not interrupted.
Every recursive frame shares the same supplied input and invocation budgets.

`FunctionIr::execute_with_services(entry_arguments, program_arguments, services, policy)`
accepts independently optional input/file capabilities in `RuntimeVmServices`.
Absent capabilities refuse in preflight, even in unreachable code, before any
service effects. `RuntimeVmFiles` adds read_text/write_text/write_text_if using
existing standard TextRead/TextWrite records and failure categories. It exposes
no input/output methods. Earlier APIs keep their file refusals.

Existing `ProcessHost` implements this narrow file trait by delegating its bounded
read, atomic replacement and conditional-write coordination operations. Passing it
explicitly grants its existing process-permission/path authority; it is not a
confined filesystem. Custom hosts own confinement, path validation, byte ceilings
and replacement/coordination correctness. No default filesystem host is installed.
Print stays memory capture; the VM never publishes output through the file trait.

Known minimum record nodes/labels are preflighted before host calls without
consuming result fuel. Cooperative checks surround each call. Returned read text
above the existing 16 MiB ceiling refuses; otherwise complete returned labels/text/
error payloads are atomically admitted before record construction. Existing input
arguments remain separately charged. Unknown returned payloads can exhaust fuel
**after** a read or write, and cancellation can refuse after a completed write.
Effects persist: there is no rollback, transaction, interruptible blocking I/O or
new cross-host atomicity promise. File-host allocations are outside logical fuel.

`RuntimeVmPolicy` selects shared instruction/output/materialization allowances,
cooperative control, a call-frame allowance (entry included, maximum 256) and IR
validation limits. Zero allowances are real. The clock starts before validation;
cancellation/elapsed checks occur before and after validation, during frame entry,
at dispatch/administrative boundaries and before successful completion. Helpers,
value walks, cleanup and trusted host work are not hard-interrupted. This is not
a process sandbox, physical heap bound or hard deadline.

Each dispatched function **and** expression instruction consumes one instruction
unit before effects. Administrative root completion consumes no instruction unit.
These units belong to this versioned IR and are not asserted to equal the number
of legacy IR instructions for the same source. Root/frame budgets are shared
across self-calls, never reset or refunded. `RuntimeVmOutcome.remaining` reports
remaining component allowances after success.

Common materialization events preserve model-v1 costs: literals/operation results
allocate their existing nodes/text, reads and entry arguments snapshot full trees,
constructors transfer children, stores/returns transfer values, numeric widening
costs zero, and releases never refund. Enum wrappers/labels and payload extraction
use the reviewed model-v2 table. Tag-only dispatch creates no VM Bool; equality
creates one. Indexed replacement transfers the existing evaluated array wrapper
and replacement without charging a second wrapper or copying child payloads.

Print preflights UTF-8 output including one LF, then admits one Unit result plus
display text and commits output before formatting/capture. String conversion
instead creates one String result. Checked integer overflow, division by zero,
array bounds, argument arity/types, foreign enum IDs and depth violations refuse;
no unlimited or alternate engine is used. Output capture and temporary/index data
are not a physical-memory accounting system. Earlier legacy APIs/model markers
remain unchanged.

Array slicing uses checked half-open bounds, including typed empty results. Append
and concat preserve first-array-directed typing and existing checked numeric
widening. Results copy selected descendants into independent flat arenas: one
result wrapper plus full selected node/text footprints, without charging input
wrappers as children. Container/bounds/length validation and atomic admission
precede payload copies. Argument evaluation remains separately charged. See the
[slice](array-slice.md) and [growth](array-growth.md) contracts.

Pure string adapters reuse existing standard helpers and preserve their grammar/
delimiter/line/control semantics. Split/parse results retain the public
`std::SplitOnce`/`std::ParsedInt` record identities and fields. Result records admit
all wrapper/child nodes, owned record/field labels and text payloads atomically
before construction. Terminal escaping measures the complete output before
admission; helper scratch remains outside logical fuel. `println` uses the same
UTF-8-plus-LF output accounting as print. Unicode length follows existing UTF-8
byte semantics; the initial v2 scalar-count discrepancy is corrected.

Complete runtime/host effect qualification, broader
control/error/deep-frame testing, Rust/JavaScript/Worker/native parity, full source/
module/program parsing, packages and tooling remain open under the
[approved completion contract](../design/G13_COMPLETION_CONTRACT.md).

[Checked program calls](program-source-v2.md) now carry source-owned function IDs
through mapping and IR. Standalone VM execution preflight-refuses those targets,
even in unreachable code. The separate [checked-program runtime](program-execution-v2.md)
now supplies the complete owned table and executes those targets with shared
policy/services and callee-owned graphs.

ProcessHost also explicitly implements RuntimeVmInput by delegating the existing
bounded strict-UTF-8 line reader. Passing it grants process stdin only. The
[experimental CLI host flags](experimental-v2-hosts.md) select these input/file
interfaces independently; neither is installed by default.
