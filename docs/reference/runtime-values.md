# Runtime compound value traversal

Status: **Implemented G13 hardening; Linux regression evidence.** Arrays and
records retain the logical snapshot semantics of
[ADR 0012](../adr/0012-compound-value-semantics.md). This changes traversal and
cleanup, not source syntax, value identity, ownership rules or runtime limits.

Rust `Value::clone` now traverses arrays/records iteratively. Scalar copies retain
a direct path. Rust display and emitted JavaScript's `svrDisplay` use explicit
work lists, preserving element/field order, separators, scalar rendering, Unit
and empty-container text. JavaScript copying and source equality already use
iterative traversal. Large displayed values no longer require recursive traversal
or repeatedly concatenating a whole child string at each nested level.

The interpreter iteratively releases runtime-owned compound values on ordinary
frame return, discarded results, overwritten names/array elements and tested error
paths. Private value/argument guards preserve cleanup when a fallible operation
returns early. These paths cover malformed binary/index/field/widening operations,
invalid builtin arguments, duplicate fields and host-output errors. Function
arguments and returned values preserve existing snapshot behavior. No partial
execution is reclassified as a successful run.

## Rust API compatibility

`Value` retains movable public enum fields. Adding an automatic `Drop` implementation
to that public enum would prevent existing downstream moves out of variants;
private runtime guards avoid that compatibility break. Rust callers can explicitly
consume their own deep values with `Value::drop_iterative(self)`. Its traversal
requires no recursive drop of compound descendants.

Ordinary Rust destruction of caller-owned `Value`/`StructValue` trees and their
derived `Debug`/`PartialEq` retain their existing behavior. Use the explicit cleanup
method for deep caller-owned values. Runtime language equality uses its separate
iterative implementation, including existing numeric and nominal record rules.
No guarantee is made about arbitrary Rust host code, allocator failures or panics.

## Evidence and remaining limits

`tests/runtime_deep_values.rs` isolates stack-overflow regressions in child test
processes. Before the fix, cloning a 16,000-level array aborted a child with stack
overflow. Tests now exercise deep arrays and alternating arrays/records across
copy, display, discard, replacement, return, user calls and error cleanup. Emitted
JS helpers also copy/display 16,000-level values under Node's 256 KiB stack setting.
A source-level recursive record/array fixture checks snapshot equality and exact
output in both engines. Actual Linux native execution is a separate smoke check.

Work lists and output buffers consume heap memory proportional to the traversed
values and output. [Explicit instruction/output allowances](execution-limits.md)
now bound dispatched instructions and printed UTF-8 payloads when selected by a
Rust host or JS generator caller. Total heap allocation, helper/host work,
hard interruption and process isolation remain separate resource-policy work.
Optional [cooperative Rust control](execution-control.md) checks cancellation and
elapsed time at interpreter boundaries and preserves iterative cleanup. Stack-safe traversal
is not a memory/CPU sandbox or qualification of Windows/macOS/mobile/browser hosts.
Compiler type depth and VM function-call depth remain distinct existing limits.


Rust hosts can also call `Value::logical_footprint()` to inspect checked u64 node,
owned-text and display-byte measurements without copying payloads or constructing
compound display text. [Footprint measurements](value-footprint.md) are read-only;
separate [opt-in materialization fuel](materialization-limits.md) now enforces
the approved cumulative logical node/text model. Physical heap remains unmetered.
