# Versioned match syntax

Status: **Partial E1 syntax foundation.** `compiler::v2::parse_match_statement`
parses exactly one standalone statement:

```svr
match outcome {
    lib::Outcome::Text(text) => { print(text); },
    lib::Outcome::End() => { return; },
}
```

The scrutinee is a bound identifier; patterns name a qualified enum and one
variant, followed by zero or one payload binding. Every arm has a braced ordinary
core statement body. Commas separate arms; the final comma is optional. Guard,
wildcard, alternative and nested patterns are unavailable. Nested match statements
in bodies await versioned body parsing; ordinary `if`/`while` bodies are parsed.
Names such as `match` and `enum` remain contextual identifiers in v1 programs.

The result owns syntax and original byte spans, including body expressions and
payload bindings. Strings/comments and CRLF/Unicode keep lexer/parser semantics.
Empty and duplicate arm lists are syntax, not proof of valid coverage. Arm shapes
can be passed to the [nominal coverage checker](enum-types-v2.md). Unresolved
body names are retained; body parsing does not establish binding mutability,
name/type correctness, return coverage or execution.

`MatchSyntaxLimits` admits source bytes, tokens and lexer/body diagnostic counts
through `FrontendLimits`. Zero diagnostic allowances use the existing minimum of
one refusal diagnostic. It separately bounds arm count before body parsing and
cumulative retained header text (scrutinee, enum/variant and binding names,
including qualified separators within enum names). Arm body storage is bounded
by admitted source/tokens, not the header-text counter. The existing fixed
structural depth guard runs before recursive body parsing. Exhaustion returns
errors without a partially successful statement. Limits do not measure process
heap or impose hard interruption.

Diagnostics preserve existing lexical/core-body codes. `E2010` reports match
grammar errors, `E1003` resource admission and `E2007` structural depth refusal.
The ordinary parser, CLI, public v1 AST/IR/Value and native format remain unchanged.
A separate [closed-scope body checker](match-bodies-v2.md) now validates ordinary
arm bodies against supplied locals. Executable enum/match source compilation,
whole-function checking and runtime/backend integration remain open under the
[approved E1 contract](../design/G13_COMPLETION_CONTRACT.md).
