# Type-only inline modules in experimental v2

```svr
mod models {
    struct Item { value: Int }
    enum Outcome { Value(Item), End }
    type Result = Outcome;
}
fn main() {
    let result = models::Result::Value(models::Item { value: 7 });
    match result {
        models::Result::Value(item) => { print(item.value); },
        models::Outcome::End() => { print("end"); },
    }
}
```

The checked v2 source pipeline admits one level of inline `mod` containing enum,
alias and record declarations. Executable functions remain at the source root.
Qualified constructors, aliases, annotations and record values use the module's
canonical type identities. Module-local type references reuse existing type-scope
resolution. Same-named root and module types, and types in different modules,
remain distinct. Unused type errors, duplicate modules and alias cycles still
refuse the whole check.

This follows existing same-file qualified type lookup, including declarations
without export markers. It is not an imported public package interface or a new
visibility/authority mechanism. Export intent does not establish package linking.
[Checked module functions](module-functions-v2.md) are now available through the
program API and v2 CLI with qualified calls and private/exported filtering.
Nested/exported module containers, imports and arbitrary module statements still
refuse. The single-entry library API remains strict; complete package visibility
and full root-type shadowing are not implied.

## Source ownership and admission

The syntax projector retains original global token coordinates and module wrappers
rather than concatenating sources or masking declarations with whitespace. Core
module alias/record headers feed the existing checked type environment; enum
fragments carry explicit module context with their original syntax spans. Checked
bodies and generated IR remain versioned, never projected as a successful v1 AST.

`ParsedProgramSyntax::enum_modules()` yields module paths parallel to its raw
`enums()` list, whose names remain written/unqualified. Context paths use shared
immutable strings rather than copying a long prefix for each enum. Declaration
coverage now includes `EntryDeclarationKind::Module` containers and their type
children; containers and children share the existing declaration allowance.
Root-function limits remain unchanged.

Before scope elaboration can repeatedly qualify names, each module charges its
UTF-8 prefix length times one plus the number of identifier tokens in its body.
The cumulative qualifier-work ceiling uses `EntrySourceLimits.types.max_text_bytes`.
This conservative admission includes identifiers that might not need qualification;
it bounds new prefix amplification before type resolution copies names. Existing
frontend, nesting, enum, context-AST and retained type-text/node limits still apply.
Logical quotas do not establish hard process memory or time isolation.

## Commands, formatting and IDE scope

Explicit v2 check/run/test/IR, guarded single-entry package/lock selection and
source-backed native builds share these types. The native runner still rechecks
embedded source at startup and grants no stdin/file services. Manifest-registered
module files and dependencies remain refused by the existing v2 package gate;
an inline type module is part of the already captured entry file.

The v2 formatter preserves module/type tokens, comments, scope and span-free syntax
through its existing safety/fixed-point gates. It does not execute source or turn
syntax-only success into semantic validity. Standalone v2 IDE mode recognizes
`mod` as a basic keyword and projects module-qualified enum/variant names through
checked IDs and original declaration ranges. Constructor/pattern references point
to the owning module's canonical variant, including aliases. Outlines remain flat;
module containers, alias/record declarations, type annotations, fields and package
contexts have no additional navigation evidence in this slice.

See the [runnable example](../../examples/type-modules/README.md). Focused tests
cover qualified execution, original Unicode/CRLF spans, nominal separation,
malformed/unsupported modules, unused type errors and exact qualifier/declaration
limits. Actual CLI/formatter, LSP, Linux VS Code development-host and relocated
Linux native qualification track their own artifacts. Installed and other-host
acceptance remains separate. G13 remains partial; nested modules, imports and later library phases still
need implementation and qualification.

## Module function bodies: syntax-only preparation

`parse_program_syntax` now retains function syntax from one-level modules as well
as the root. `functions()` contains original written names and bodies in source
order; `function_modules()` supplies parallel immutable module paths (None for
root functions). Same-named functions in distinct modules remain distinguishable
without rewriting names or coordinates. Export intent and recursive match/control
bodies remain in the versioned syntax. The core v1 header projection still contains
only type context, never a successful old-AST projection of these bodies.

The raw function allowance is shared across root and module bodies, alongside the
existing declaration, frontend, body/depth and qualifier-work ceilings. Syntax
errors still refuse the whole parse. Module paths share immutable strings rather
than copying prefixes per body. This API returns no checked identity, scope/type
proof or executable function table.

The v2 formatter (including supported package selection) can prepare such drafts
with its token/comment/AST/fixed-point and guarded-write gates. It can format a
syntactically valid body containing semantic errors. `fmt --check` checks layout,
not compilability. IDE formatting uses this syntax path while diagnostics and
navigation continue to use checked-source admission.

`check_entry_source` remains strict. The program API, CLI and native source
rechecking now support the [checked module-function subset](module-functions-v2.md).
Raw syntax alone still establishes no semantic or executable proof: invalid bodies
continue to fail every checked/execution path and publish no navigation facts.
