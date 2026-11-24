# Checked same-file module functions in v2

```svr
mod values {
    enum Outcome { Value(Int), End }
    type Result = Outcome;
    fn hidden(value: Int) -> Result { return Result::Value(value); }
    export fn obtain(value: Int) -> Result { return values::hidden(value); }
}
fn main() {
    let result = values::obtain(7);
    match result {
        values::Result::Value(value) => { print(value); },
        values::Outcome::End() => { print("end"); },
    }
}
```

`check_program_source` and the explicit v2 CLI now check, lower and execute
functions in one-level inline modules. Function bodies retain written names,
annotations and global spans; the program owns canonical `module::member` IDs.
Forward/mutual/recursive qualified calls share those IDs with lowering and the VM.
All bodies are checked, including unused private functions; semantic failure
never publishes a partially checked/executable program.

## Calls and visibility

Call module functions with qualified names such as `values::hidden()`.
Unqualified function calls preserve the existing root/standard lookup rules;
they do not gain implicit module-member resolution. Within the defining module,
qualified calls can use private or exported members. Root functions and other
modules can call only exported members. A forbidden private call is unavailable
to ordinary call checking (E3004). Root functions remain available by their normal
names in same-file module bodies. This is source call visibility, not an OS
sandbox or package interface policy.

Module-local enums, aliases and records resolve in the defining function's type
context. Callee signature annotations are canonicalized in the callee's defining
context before another module consumes them. Two modules can own different types
with the same written name. To prevent a scoped lookup from reinterpreting a root
canonical type, this first checked profile rejects a module-function context that
shadows a different root type (E3120 at that function). Data-only modules retain
their existing homonym support. Full root/module type shadowing remains an open
integration gate rather than silently selecting a nominal type.

Standard callable identities remain reserved. An enum constructor/alias whose
qualified name collides with a declared function also refuses with E3110; no
constructor table entry silently replaces a function target. Private/exported
visibility cannot grant input/filesystem services or alter the selected VM policy.

## Entry, testing and inspection

Only a root `main` is the application entry. A module member named `main` is an
ordinary function and can have ordinary parameters/results. It never substitutes
for a missing root main in run/native builds. V2 test discovery remains root-only:
module `test_` members are checked as functions but not automatically selected.
Root tests can call exported module functions.

`function_id("module::member")`, `function_id_at(index)` and `function_name(id)`
inspect canonical program identities. Trusted embedding code can select an owned
function ID; source visibility is not a capability restriction on that embedding
API. Linked execution verifies each graph's owned ID and original source span,
not just a possibly repeated short name. IR inspection and symbol declarations
use canonical names; hover signatures retain written source annotations. Checked
call references keep original qualified spans and navigation targets. Outlines
remain flat and now include checked module containers with original source ranges. The earlier single-entry `check_entry_source` API stays strict and
does not acquire module-function linking.

## Admission and supported scope

Module type views share immutable nominal/record facts and retain one charged
known-type view per module, reused by its functions. Copied keys/types, module
names/prefixes/cache labels and canonical signature text consume the existing
cumulative program evidence/text ledgers before copying. Visible callable headers,
constructor aliases and semantic facts retain their existing count/text quotas.
Exact and one-under tests cover these aggregates. The same runtime budgets,
call-depth limits and explicit host policy apply across every module frame.
Compiler logical admission is separate from execution time and is not hard
process memory/time isolation.

Qualified functions work through v2 check/test/run/IR, formatting, captured
single-entry packages/locks, checked standalone navigation and source-backed
native builds. Native executables still embed/recheck bounded source at startup;
this is not an executable IR codec or optimizing backend. Nested modules,
imports, registered source files/dependency linking, full shadowing, package
visibility and type-annotation/field navigation remain open.
No default legacy command or v1 public AST/value variant changes.

See the [runnable example](../../examples/module-functions/README.md). Focused
checks cover private access, recursion, same-named members/types, signature scope,
root entry/test selection, collision refusal and aggregate quotas. Actual CLI,
LSP/VS Code and relocated Linux native acceptance track their precise artifacts;
installed and other-host qualification remains separate. G13 is still partial.
