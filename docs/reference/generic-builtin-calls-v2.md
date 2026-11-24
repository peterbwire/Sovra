# Host-free standard calls in generic programs (V2)

`compile_generic_builtin_source` extends the explicit adapter source environment
with two compiler-owned calls. `--experimental-generics` now selects this profile.

| Call | Arguments | Existing result contract |
| --- | --- | --- |
| `std::parse_int` | one String | `std::ParsedInt { ok:Bool, value:Int }` |
| `std::split_once` | two Strings | `std::SplitOnce { found:Bool, before:String, after:String }` |

The decimal call reuses the existing core strict parser: no whitespace, plus sign,
leading zeroes, nondecimal digits or overflow. Failure has `ok=false,value=0`.
The split call preserves the first delimiter match, Unicode boundaries and the
existing empty-delimiter semantics. Absence has `found=false,before=input,after=""`.
Existing builtin return types are unchanged. Explicit [source adapters](generic-result-adapters-v2.md)
convert these records to Option/Result; `?` then composes normally.

Symbolic checking admits only the two exact qualified names with no type arguments
and exact String arity/types. Their results require immutable compiler-owned
standard schemas. Shared call-count/argument-edge/type-generation/work admission
applies across all bodies, including unused functions. Builtin facts do not invent
specialized Sovra function bodies or callee graph edges. Concrete rechecking
independently verifies argument types and result identity. IR independently checks
result field layouts and argument stack types before execution.

The VM charges instruction fuel, input scanning, copied strings, scalar/result
nodes and nominal labels to existing shared budgets. String allocation follows
work and materialization admission. Decimal scanning charges input bytes. Split
scanning conservatively admits `(input bytes + 1) * (delimiter bytes + 1)` work,
with checked arithmetic, before calling the host-free string operation. Large
inputs can therefore refuse even when a particular search would be fast. Existing
cooperative cancellation/elapsed checks surround scanning and copying; this is a
logical budget, not a hard process-time sandbox. Refusals retain the original call
range and return no partial success.

Native feature word **63** repeats this profile's bounded checking at startup.
Words 1/3/7/15/31 retain their historical environments; only those six exact words
are accepted. Unknown/partial words and builtin consumer downgrades refuse.
Ordinary source APIs and the schema-only adapter API remain strict for these calls.
The [consumer example](../examples/generic-result-adapters.svr) now uses real parsing
and splitting through explicit adapters, including a native locked-package test
run without source/manifest/lock or tool PATH on Linux.

This adds no filesystem, console, process or network access. Other builtin calls,
remaining E2/G13 contract work, installed-payload execution and other-host
qualification remain open. Task Manager is unchanged.

String utility update: [checked String calls](generic-string-builtins-v2.md)
now extend experimental commands and native feature 127. Earlier source APIs and
image words retain their previous environments; host I/O remains gated.
