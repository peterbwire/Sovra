# Structured standard errors

The complete [generic source pipeline](generic-source-v2.md) now provides a
compiler-owned nongeneric record `std::Error`:

| Field | Type |
| --- | --- |
| `code` | `String` |
| `message` | `String` |
| `offset` | `std::Option<Int>` |

Construct it with the exact fields, in any initializer order:

```svr
std::Error {
    code: "parse",
    message: "invalid input",
    offset: std::Option::<Int>::Some(7)
}
```

Use `std::Option::<Int>::None()` when no offset is available. The record itself
has no type arguments; `std::Error::<...>` refuses. Code/message text and offset
values are data, not an exception or host operation. No domain constraints on
code text or offset values are invented by the record type.

The [runnable example](../examples/generic-error.svr) creates an error inside a
generic `std::Result<T,std::Error>`, exhaustively matches it, reads the fields and
checks the offset before silently succeeding:

```sh
svr check --experimental-generics docs/examples/generic-error.svr
svr run --experimental-generics docs/examples/generic-error.svr
svr build --experimental-generics --emit exe --output app docs/examples/generic-error.svr
```

The same source supports a [guarded root package](generic-packages-v2.md), source
locks and host-native packaging. Field initialization, access, equality and
whole-value reassignment reuse the bounded record pipeline. Existing copies and
payload bindings preserve owned snapshots when a local is subsequently replaced.
Missing/duplicate/unknown fields, incompatible String/offset types, forged
qualified declarations and interchange with user record types refuse. Ordinary
user `Error<T>` remains available. General nongeneric user record syntax is still
outside the generic source profile.

A private finite parser adapter admits only the compiler's fixed record template,
with an empty parameter owner and immutable standard-error marker. Public generic
record parsing stays strict. `is_standard_error()` exposes the marker and
`qualified_name()` gives `std::Error`; short syntax names and fragment spans refer
to the synthetic template, never a user file. The standard definition consumes
registry/definition/member/source/annotation/work admission even when unused.
User declaration listings and source syntax quotas remain about original input.

The constructor validates the exact original `std`, `::`, `Error`, `{` tokens,
including grouped/trivia-containing expressions. It admits one zero-argument
nominal type head under the existing per-input and cumulative source/token/node/
name/depth/work limits, then shares nominal expansion and identity validation.
No written generic list or arbitrary qualified record authority is fabricated.
The head costs its original byte range, four type tokens including EOF, one node
and ten canonical name bytes. Fields and generated substitutions have their own
existing cumulative admission. VM materialization charges qualified nominal and
field labels plus actual payload text under the existing logical fuel rules.
These limits are logical controls, not a physical heap bound or process sandbox.

New generic native images use feature word 7: base source bit 1, standard enums
bit 2, and standard Error bit 4. The decoder retains word 1 without standard
definitions and word 3 with Option/Result only. Word 5, standalone dependency bits
and unknown bits refuse. Old profiles cannot acquire Error through modified
source alone. Header version/size, source ceiling, fixed runtime policy and
legacy/plain-v2 format separation remain unchanged. Images still contain bounded
source and the Rust VM, with complete startup rechecking.

`tests/generic_error_v2.rs` covers generic error payloads, field rules, namespace
separation, grouped/commented heads, aliases and snapshots, provenance, exact
constructor syntax admission, materialization refusal and native compatibility.
`tests/generic_error_cli_v2.rs` checks the documented source and a real locked
native package after its inputs are removed, with an empty PATH. Native/package
qualification remains Linux only; other-host and installed-runtime qualification
remain separate gates.

Existing specialized builtin results retain their return types. Explicit adapters,
postfix `?`, dependency/module linking, JavaScript/tooling and later G13 library
work remain open. This slice does not complete G13 E2 or G13.
