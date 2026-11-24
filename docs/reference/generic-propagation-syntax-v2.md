# Syntax-only postfix propagation capture

`parse_generic_propagation_block` parses exactly one complete `{...}` block and
returns owned `GenericPropagationSyntax` with original-source marker ranges. It
is a Rust syntax API, separate from [checked propagation compatibility](generic-propagation-v2.md).
It does not type-check, insert returns, lower instructions or enable question
marks in executable source/CLI/native modes.

For example, parsing `{let value=take(first?,other()?)??;}` retains four sites:

| Operand | Resulting expression |
| --- | --- |
| `first` | `first?` |
| `other()` | `other()?` |
| `take(first?,other()?)` | `take(first?,other()?)?` |
| `take(first?,other()?)?` | `take(first?,other()?)??` |

Each `GenericPropagationSyntaxSite` retains the whole operand span, the exact
single question token and the resulting expression span. Sites follow parser
application order: nested operands precede the containing marker. This order is
syntax metadata, not a promise that conditional branches execute. Fields, indexes,
calls, grouping, generic constructors, local annotations, loops/branches and
match arms share the same private capture. Comments/trivia and Unicode/CRLF byte
and character coordinates remain original. String/comment question characters
are not sites. Repeated `?` is captured as repeated postfix application, not a
new null-coalescing operator.

The adapter uses a private lexer mode and parser sidecar. Ordinary tokenization
still reports E1000 for an unquoted question mark. Ordinary parser constructors
leave capture disabled, including public token-stream parsing. No public AST
variant or field changes. The adapter discards its private temporary AST and
publishes only source-owned metadata, so an unwrapped legacy AST cannot be sent
to an old checker/backend as successful propagation. Existing generic-source
compilation and native feature words 1, 3 and 7 remain strict.

Speculative indexed-assignment parsing rolls back captured sites together with
explicit-call facts before retrying an expression statement. Thus `array[index?];`
contains one site, not two, while `array[index?]=replacement?;` retains both actual
source sites. A question-mark expression cannot be treated as an assignment
statement target. Prefix/nonexpression/type-position markers, incomplete blocks
and trailing source refuse without partial successful metadata. Every retained marker must map uniquely to the exact original question token,
and every original question token must have a marker. Missing/duplicate coverage
refuses at the original token, including otherwise opaque generic type lists.

`GenericPropagationSyntaxLimits::DEFAULT` admits 128 KiB original source, 32768
non-EOF tokens, 64 diagnostics, 4096 sites, at most 128 question applications in
one postfix chain, and 1000000 logical work units. Chain profiles above 128 refuse.
Input bytes, tokens including EOF and marker validation are admitted before
capture allocation; defaults and exhaustion never select an unlimited fallback.
Existing delimiter/parser nesting guards remain in force. Work is logical input/
metadata accounting, not a hard parser CPU or physical heap bound. Typed expression
depth and concrete control-flow/resource obligations still require their own
integration. Zero marker/chain allowance can admit a block without question marks.

Syntax can retain `1?` or an unresolved `missing?`; that is not a valid Option/
Result typing proof. [Checked compatibility](generic-propagation-v2.md) remains
necessary, together with source/body binding, concrete rechecking, payload snapshot
policy, early-return lowering, temporary cleanup and native feature gating.
Executable postfix propagation, adapters and G13 E2 remain incomplete.

`tests/generic_propagation_syntax_v2.rs` covers nested/chained original ranges,
field/index/call/group suffixes, speculative rollback, loops/matches/records,
Unicode/CRLF/trivia, malformed/type-position markers, exact input/work/site/chain
boundaries, old-mode refusal, source retention and every UTF-8 prefix of a small
fixture. Existing parser, lexer and CLI regressions verify the default paths.

The [opt-in symbolic body checker](generic-propagation-bodies-v2.md) now consumes
this capture and validates active operand types. Ordinary source/CLI/native
paths remain strict until concrete/executable integration is complete.
