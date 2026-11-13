# G13: first payload-enum syntax slice for error handling

Status: **Proposed for review; not implemented.** This begins the error-handling
work in the [18-task checklist](../18_TASK_COMPLETION_CHECKLIST.md), after the
implemented array APIs. It does not implement exceptions, Option/Result or enum
execution. Existing specialized InputLine/TextRead/TextWrite/ParsedInt records
remain compatible and usable.

## Direction and bounded first slice

Ordinary errors eventually need explicit success/failure values with typed
payloads and exhaustive handling. The recommended path is nominal payload enums,
then typed constructors and exhaustive matching, then reviewed reusable generic
Option/Result and propagation. Do not invent implicit exceptions or stringly typed
error channels inside filesystem, HTTP or process builtins.

The first implementable slice is an additive **single-declaration enum syntax API**.
Current executable Program has no enum declaration field. G11's data inspector
retains only enum headers and explicitly unparsed bodies. Its application enum
examples must not silently acquire new executable meaning. This new API captures
variant bodies before changing those older APIs, CLI modes or runtime formats.

The lexer already recognizes =>, while enum/match are currently contextual
identifiers. Keep ordinary lexing and identifier compatibility unchanged. New
syntax must not globally reserve names or accept old application bodies as typed
errors. Match parsing/execution is outside this first slice.

## Proposed accepted declaration grammar

```svr
enum ReadOutcome {
    End,
    Text(String),
    Failure(String),
}
```

Accept exactly one enum declaration, optionally prefixed export, with comments/
whitespace and an optional trailing comma. Variants are comma-separated identifiers,
either nullary or with exactly one parenthesized type annotation. Reuse existing
bounded primitive/qualified-name/[T] type spelling, including nested arrays and
aliases syntactically. Names/types retain exact original UTF-8 source spans.

Export is captured as syntax only; context/visibility rules are not decided by a
standalone declaration parser. Payload type names remain unresolved. Duplicate
names and empty enums can be retained syntactically for later semantic diagnostics;
that is not a declaration of semantic validity or inhabitation rules.

Reject extra declarations or trailing nontrivia, missing separators/braces,
malformed types, empty parentheses, multiple/named payload fields, discriminant
assignments, generic parameter syntax, attributes and recovery that silently drops
variants. Unsupported forms return an error, not partial successful syntax.
No model syntax, constructors, match statements, guards, wildcard/nested patterns,
function/closure payload syntax or generic substitution is added.

## Additive Rust API

Add compiler::sum_syntax with:

- `parse_enum_declaration(source, limits) -> Result<EnumDeclarationSyntax, EnumSyntaxError>`.
- `EnumDeclarationSyntax`: export flag, owned enum name, exact name/declaration
  spans and ordered Vec of variant syntax.
- `EnumVariantSyntax`: owned name, exact name/variant span, optional owned payload
  annotation spelling and exact annotation span. Nullary variants have no payload.
- `EnumSyntaxLimits`: explicit independently bounded source bytes, tokens, variants,
  cumulative payload-type nodes and retained name/type UTF-8 bytes; a documented
  DEFAULT profile. Zero is a real limit, not unlimited.
- `EnumSyntaxError`: distinguish lexical/syntax Diagnostics from an explicit
  metadata/resource limit error; expose no partially successful declaration.

Use Debug and appropriate clone/equality implementations, keeping ownership and
public fields/types explicit in the API reference before claiming implementation.
Do not add fields to public Program/Module, new public Statement/Expression/Value
variants or serialized native opcodes in this slice. Existing parser/checker/
compiler/formatter/IDE signatures and behavior stay strict and unchanged.

Lexical/syntax errors reuse existing lexer/parser codes and original spans; do not
fabricate executable semantic diagnostics. Limit failures are structured metadata
errors with their dimension, never success-with-missing-variants or a fallback to
unbounded parsing. Refusal is not a whole-project checker success.

## Admission and accounting proposal

DEFAULT for this opt-in API: 1 MiB source, 100,000 tokens, 4,096 variants,
16,384 payload-type nodes and 256 KiB retained name/type bytes. Reuse the existing
128-level structural type depth bound. These are new API defaults, not changes
to existing compiler or command limits. Each field accepts an explicit usize cap.

Check source bytes before scanning. Use existing bounded token admission so a
token limit is not checked after constructing an unbounded token vector; count
nontrivia non-EOF tokens consistently with the existing lexer API. The retained
token vector may contain one additional EOF sentinel, which consumes no token
allowance; whitespace and line comments consume no tokens.
Admit variant metadata and owned names/annotation strings before retaining them.
Count one node per payload leaf type plus each array wrapper; qualified path
components contribute text, not separate type nodes. Nullary variants consume no
payload-type nodes. Count enum/variant names and the complete canonical retained
payload spelling once each, with checked cumulative arithmetic and no refunds.

For `enum E { A, B([Int]) }`, expected retained type nodes are 2 and retained text
is 8 bytes: E + A + B + [Int]. Verify this against actual implementation before
reporting it as enforced evidence. Token count excludes the final EOF sentinel.
An empty variant list needs zero variant/type-node allowance but still pays its
enum name/text/tokens. Comments are not retained name text. Source-byte admission
remains independent. Admission errors discard all intermediate owned metadata.

Use borrowed source/token access where existing APIs permit. Do not copy the whole
source into each syntax node or introduce unbounded recursive traversal. Resource
limits describe this metadata, not process RSS, elapsed time or a source sandbox.

## Acceptance before marking this slice implemented

1. Valid nullary/single-payload, export/comment/trailing-comma, primitive/qualified/
   nested-array/alias spellings retain order and exact spans. Duplicate syntax is
   retained for later semantic checking, not silently deduplicated.
2. Invalid/unsupported syntax fails explicitly, including trailing declarations,
   tuple/named payloads, generic/discriminant forms and unterminated input.
3. Exact/one-under/zero source/token/variant/type-node/text limits and arithmetic
   checks publish no partial syntax. Confirm shared depth boundaries and Unicode/
   CRLF/source positions without changing identifier rules.
4. Ordinary parsing, named identifiers, G11 enum-header/unparsed-body coverage,
   checked stages, package locks, IR/runtime/native formats and defaults retain
   compatibility. Add no implicit CLI opt-in or executable enum claim.
5. Update reference/design/checklist status with actual targeted/full regression
   evidence. No native/browser/mobile/MSRV/release qualification is inferred from
   a pure syntax API. No new dogfood application is needed for a non-executable
   metadata API; independent package consumers become mandatory at execution.

## Later semantic and runtime gates

Subsequent review must define nominal enum identity and exports, constructor
resolution and payload coercion, exhaustive handling/return flow and scoped pattern
bindings. Generic error values, recursive enum graphs and propagation require
explicit decisions. Runtime representation must preserve public-stage contracts
or obtain a separately reviewed compatibility migration, including native version
handling, snapshots/equality/display, iterative cleanup, complete node/text fuel
and Rust/ordinary Node/worker parity. Do not assume a record tag is sufficient for
source type safety or authorize exceptions by parsing a variant body.

The first syntax API is deliberately labelled partial and does not close G13/G11.
Error values remain unavailable until those later checked/runtime gates pass.
HTTP/resource/process work must not claim those prerequisites from syntax alone.

## Approval boundary

[AGENTS.md](../../AGENTS.md) says: "Document and pause before fundamental syntax,
memory-model, type-semantics or compatibility decisions." The approved
[collection/data plan](G13_COLLECTIONS_AND_DATA.md) also requires review before
the next error-model decision. Approval requested here covers this exact additive,
non-executable parser API, grammar and admission table. It does not approve new
runtime values/IR, a public AST break, generics, match execution or migration of
existing standard result records. Earlier array/resource approvals remain valid.
