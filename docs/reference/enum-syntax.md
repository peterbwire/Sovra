# Bounded payload-enum declaration syntax

Status: **Implemented additive Rust syntax API; not executable enums.**
The approved [first error-handling contract](../design/G13_SUM_TYPE_SYNTAX.md)
adds compiler::sum_syntax without changing ordinary parsing or project defaults.

```rust
use sovra::compiler::sum_syntax::{parse_enum_declaration, EnumSyntaxLimits};
let declaration = parse_enum_declaration(
    "enum ReadOutcome { End, Text(String), Failure(String), }",
    EnumSyntaxLimits::DEFAULT,
)?;
assert_eq!(declaration.variants[1].payload_type.as_deref(), Some("String"));
```

The function accepts exactly one declaration, optionally prefixed export, with
existing line comments/whitespace and optional trailing comma. Each comma-separated
variant is a name, optionally followed by one parenthesized type annotation.
Annotations use existing primitive/alias/qualified-name/[T] spelling; names remain
unresolved. Duplicate variants and empty enums are retained as syntax, not declared
semantically valid. Export context and nominal identity are not checked here.

Extra declarations/trailing tokens, missing separators, malformed annotations,
empty parentheses, tuple/named payloads, discriminants, generics, attributes and
function payload syntax reject the request. There is no recovery that drops a
variant and returns success. enum/match remain contextual identifiers in ordinary
lexing; no new global keyword is reserved.

## Owned results and spans

EnumDeclarationSyntax has is_exported, owned name, name_span, declaration span
and an ordered Vec of EnumVariantSyntax. Each variant has owned name, name_span,
variant span, optional owned payload_type and optional payload_type_span.
Canonical payload spelling removes trivia, retaining brackets and :: separators.
For example, `[ [ lib :: Alias ] ]` is retained as `[[lib::Alias]]`.

All spans refer to the original source: byte offsets plus zero-based lexer
line/scalar-column positions. Declaration span runs from export/enum through the
closing brace, excluding surrounding trivia. Variant span excludes its separator;
nullary variants span only their name. Payload span excludes parentheses but
includes original internal whitespace/comments. No source copy is retained in
each node. Result strings are owned and independent of the input lifetime.

## Explicit limits

EnumSyntaxLimits is Debug + Clone + Copy + Eq and exposes DEFAULT:

| Field | Default | Count |
| --- | --- | --- |
| max_source_bytes | 1 MiB | Source UTF-8 bytes, checked before lexing. |
| max_tokens | 100,000 | Nontrivia non-EOF tokens, admitted by the existing lexer. |
| max_variants | 4,096 | All ordered variants, including duplicate names. |
| max_payload_type_nodes | 16,384 | One leaf plus each array wrapper, cumulatively. |
| max_retained_text_bytes | 256 KiB | Enum/variant names and canonical payload strings, once each. |

Zero is a real cap. Caller-selected caps use checked usize arithmetic with no
refunds or unbounded fallback. Qualified components are text, not additional type
nodes. Comments contribute source bytes but not retained name text. The token
vector includes one uncharged EOF sentinel. Lexer token payloads are bounded by
source/token admission separately; retained-text accounting is for returned
metadata, not every temporary allocation or process RSS.

Shared type depth includes the leaf: up to 127 array wrappers (128 total levels)
are accepted, while 128 wrappers fail. The existing lexer diagnostic guard is
256 errors; it is not a new configurable field of EnumSyntaxLimits. These shared
guards remain active even if callers raise other caps. Syntax errors emit one
parser diagnostic; lexical failures retain existing lexer diagnostics within its
guard. No public lexer result or error format changed.

`enum E { A, B([Int]) }` needs exactly 22 source bytes, 12 non-EOF tokens,
2 variants, 2 payload-type nodes and 8 retained text bytes. Tests admit each exact
boundary and refuse each one-under dimension. `enum E{}` needs no variant or
payload-node allowance, but still needs its name, source and four tokens.

## Failures and scope

EnumSyntaxError is Debug + Clone + Eq and implements Display/Error:

- Diagnostics(Diagnostics) preserves existing E1xxx lexical/E2xxx syntax codes
  and original locations, without inventing executable semantic results.
- Limit { dimension, limit } identifies SourceBytes, Tokens, Variants,
  PayloadTypeNodes, RetainedTextBytes, TypeDepth or LexicalDiagnostics.

Source refusal precedes scanning; lexer admission precedes metadata parsing.
Names and canonical type strings are admitted before retaining owned copies.
Limit failures discard intermediate metadata and expose no partial declaration.
This is not an end-to-end memory/CPU sandbox or an executable program check.

Nine integration tests verify original Unicode/CRLF spans, canonical types,
duplicate/empty syntax, unsupported forms, exact/zero/cumulative quotas and every
default saturation boundary. Two private tests verify overflow refusal and public
lexer diagnostic compatibility. Whole-source parsers still reject executable enum
syntax, and G11 enum bodies remain explicitly unparsed. Existing identifiers and
ordinary compilation remain compatible.

Next stages need separate contracts for nominal enum/type checking, constructors,
exhaustive matches/return flow, payload bindings, runtime representation and full
fuel/backend/package evidence. General Option/Result/propagation are later work.
No public AST/Value/IR/native format, CLI opt-in, std-result migration, browser/
mobile/runtime or new platform qualification is introduced by this API.
