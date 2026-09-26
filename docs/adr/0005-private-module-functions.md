# ADR 0005: Callable private inline-module functions

Status: Accepted; implementation added, execution verification pending.
The user directed implementation after presentation of recommended Option A.

## Previous implementation

Semantic analysis validates all module function bodies, but only exported
functions enter the callable map. IR lowering likewise emits only exports.
Every function body receives the same global callable map. Consequently a
private helper is unavailable even to another function in its own module.
This finding is based on source inspection; the current Windows Application
Control policy prevents running rustc to verify a fresh executable probe.

## Decision: Option A, explicit same-module calls

Allow any function in an inline module to call that module's private functions
using the existing qualified spelling. For example, this would become valid:

```svr
mod math {
    fn add(left: Int, right: Int) -> Int { return left + right }
    export fn double(value: Int) -> Int { return math::add(value, value) }
}
fn main() { print(math::double(21)) }
```

Expected output after implementation: `42`. Top-level functions and functions
in other modules still cannot call `math::add`. Such access retains E3004 at
the callee. Private function values remain unsupported; a visible function
name used as a value receives E3015.

Keep existing bare-name resolution: an unqualified call refers to a top-level
function or builtin, never implicitly to a module member. Require `math::add`
inside math too. Permit self-recursion and mutually recursive private helpers
subject to the existing call-depth limit. Do not introduce imports, nested
modules, cross-file loading, new syntax, or function values.

### Builtin collision compatibility

ADR 0003 currently exempts private module declarations from E3016 because they
are not callable. Making them callable creates ambiguity with builtin-first
dispatch; for example a private `std::len` cannot safely share that symbol.

Extend E3016 to all module declarations whose qualified names exactly match a
builtin. A previously accepted private `std::len` must be renamed. Other private
functions such as `math::len` remain valid. Preserve bare `print` as a builtin
alias and keep top-level collision rules unchanged. This explicitly revises
ADR 0003's private-function exemption; it must not be changed silently.

### Implementation and verification plan

1. Add negative and positive semantic regressions before implementation.
2. Build the public callable map once, then add the current module's private
   declarations when checking that module. Never add other modules' private
   declarations or expose them to top-level bodies.
3. Lower all module functions under their existing qualified names. Preserve
   compiler stage signatures and runtime dispatch. Privacy is enforced by source
   checking, not by hiding symbols in inspectable IR or emitted JavaScript.
4. Cover private-to-private calls, exported-to-private calls, recursion, sibling
   isolation, external rejection, bare-name stability, duplicate declarations,
   builtin collisions, arity/type failures, and function-value rejection.
5. Compare interpreter and JavaScript execution; cover CLI check/run/build and
   JSON diagnostic spans. Update the specification, module example/course,
   ADR 0003, status and development log. Count blocked tests as unverified.

## Option B: Retain current behavior

Keep private functions validated but uncallable until broader symbol/module
resolution is designed. This preserves ADR 0003 unchanged but leaves private
helpers unavailable. Document the limitation rather than claiming execution.

## Approval boundary

AGENTS.md requires: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." Option A changes source visibility
and rejects previously accepted private builtin collisions. The subsequent user instruction to add roadmap code selects Option A.
