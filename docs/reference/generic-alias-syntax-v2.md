# Source-owned generic aliases in v2

```rust
use sovra::compiler::v2::{parse_generic_alias, GenericAliasLimits};
let alias = parse_generic_alias("type Rows<T> = [T];", GenericAliasLimits::DEFAULT)?;
```

The additive Rust API now attaches a generic parameter owner to a complete root
alias declaration and binds parameter occurrences in its target annotation.
`GenericAliasSyntax` retains the full original source, written alias name,
declaration/name ranges, parameter/target ranges and immutable parameter-use table.
Independent alias parses cannot share parameter identities merely because names
match. Foreign parameter IDs refuse when requesting a declaration span.

This initial attachment profile accepts exactly one `type Name<T> = Target;`.
The parameter list must be nonempty and obey the existing unique-name/`std::Key`
profile. The target uses the approved qualified generic/array annotation grammar.
Supported line comments and whitespace preserve source ownership. Adjacent
`type Rows<T>=[T];` works: this declaration grammar treats the first byte of the
lexer's `>=` token as the parameter terminator. Ordinary comparison lexing and
other source/type parsers are unchanged. Extra declarations, trailing tokens,
missing components and `T<Int>` applications of a parameter refuse completely.
Exported/module aliases are not included in this initial attachment API.

## Coordinates and retained ownership

Parameter-list and target parsers keep their immutable fragment-local source and
spans. `parameter_range()` and `target_range()` identify those exact fragments in
the full alias source. `parameter_span(id)` and `target_node_span(index)` return
full-source spans; out-of-range indices and foreign identities refuse. Errors
from fragment parsing/binding are translated to the original declaration's byte,
line and column coordinates. No concatenated or rewritten source is presented as
the original. Full-source offsets are computed only from admitted owned fragments.

The bound table shares annotation/parameter arenas and sources through immutable
owners; attachment does not deep-copy them. Complete original source and both
fragment copies are charged together before retention. Declaration, parameter
and target copied names share one aggregate name allowance. Independent fragment
limits remain in force; lowering a parent allowance cannot reset a child quota.
Whole-source and fragment lexer admissions are independent bounded operations.

The default profile permits 128 KiB original input, 32,768 original tokens
including EOF, 256 KiB total retained source/fragment text, 48 KiB aggregate copied
names and 4,096 parameter-use records. Existing parameter/type fragment profiles
apply as well. Logical admission does not promise process memory or CPU isolation.
Exact and one-under tests cover the aggregate and independent allowances.

## Remaining semantics

This is declaration syntax and parameter-use ownership, not a resolved alias or
checked generic program. Unknown target names and wrong application arities may
parse successfully. The separate [prepared alias environment](generic-alias-expansion-v2.md) now
checks alias targets/cycles/bounds and expands substitutions. This syntax parser
itself does not do so. Broader visibility, code specialization and executable
integration remain open. Generic
[function header attachment](generic-function-syntax-v2.md) now retains
bounded opaque bodies; generic body checking remains open.
[Generic enums](generic-enum-syntax-v2.md) now have syntax attachment too.
[Generic records](generic-record-syntax-v2.md) now have their own bounded
syntax attachment API. Existing CLI and
source checking still refuse unsupported generic programs.

See [parameter ownership](generic-parameters-v2.md),
[type annotation syntax](generic-type-syntax-v2.md) and the approved
[G13 contract](../design/G13_COMPLETION_CONTRACT.md). G13 is partial.
