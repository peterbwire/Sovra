# Checked generic package type annotations

With [generic package diagnostics](generic-package-diagnostics.md) enabled,
successful checks now supply definitions and hover for user types in:

- Function parameter and return annotations.
- Record field annotations.
- Enum payload annotations.
- Transparent alias targets.
- Explicit local `let` annotations, including nested array and generic types.
- Explicit function-call type arguments.
- Record/enum constructor type heads and their nested type arguments.

Record, enum and alias declarations also appear in checked outlines. Imported
records/enums lead to their actual dependency source. A written alias leads to
the alias declaration, not the type produced by expanding it. Identical names
in different files retain their own nominal identities and visibility rules.
Compiler-injected types and type parameters have no fabricated source destination.

Projection resolves the retained parsed annotations under the sealed package
registry and each declaration's original parameter scope, before transparent
expansion erases alias identity. Registry IDs map to owned source declarations.
Original annotation fragments must match their source ranges exactly. Tokens
locate the terminal identifier of a qualified name, including paths containing
trivia; they do not choose a target by spelling. Unimported or private type use
still fails checking and publishes no navigation.

The existing [fingerprinted package navigation](generic-package-navigation.md)
checks apply to open and closed destinations, byte boundaries, generations,
optional quotas and all-or-unavailable publication. Type and call references
share one reference budget; all projected declarations share one declaration
budget. Failed or unavailable checks clear previous evidence. Checking never
saves buffers, emits executables or runs source.

This slice covers declaration/local annotations and explicit call/constructor
types. Type-parameter declarations have no fabricated source destination.
[Field navigation](generic-package-field-navigation.md) covers checked field reads
and constructor labels. [Package local navigation](generic-package-local-navigation.md) covers
value bindings. Existing
standalone navigation remains separate; no new setting is required beyond the
package-diagnostics opt-in.

Linux compiler/LSP tests cover imports with qualified whitespace, alias-preserving
destinations, same-named local/dependency types, definition-scope resolution of
record/enum annotations, original Unicode/CRLF positions, outlines and privacy/
import refusal. Actual Linux VS Code host acceptance now passes; see [host qualification](generic-package-ide-host.md). Additional-host runtime acceptance remains separate.

Local annotation projection uses only ranges retained by successfully checked
`let` statements. It reparses each bounded original type fragment, then resolves
it under the owning function's parameter scope and sealed per-file registry.
This preserves written alias destinations rather than guessing from the expanded
binding type. Parsing, resolution, token matching and output consume the shared
optional work/output limits; exhaustion suppresses the whole projection.

Call and constructor projection starts from ranges retained by checked expression
facts. Calls use bounded explicit-list parsing and source-order argument arenas.
Constructors rebuild the original qualified head with the existing source-backed
constructor syntax helper, preserving offsets through trivia and nested types.
The sealed owning-function scope resolves these nodes before alias expansion.
Bare type parameters and compiler-injected types have no fabricated definition.
Nongeneric calls have no explicit-list projection. Non-generic standard
constructors have no user type destination. Names in comments, string literals
and unrelated body tokens do not create navigation facts.
