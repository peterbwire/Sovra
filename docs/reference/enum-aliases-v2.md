# Transparent enum aliases in v2

```svr
enum Outcome { Value(Float), End }
type Result = Outcome;
type CopiedResult = Result;
fn main() {
    let result = CopiedResult::Value(7);
    match result {
        Result::End() => { print("end"); },
        Outcome::Value(value) => { print(value); },
    }
}
```

The experimental v2 pipeline now accepts an enum alias as a constructor or
match-arm qualifier. Alias chains retain the original enum/variant identity;
they never declare another nominal enum. Canonical and alias spellings can be
mixed in exhaustive matching. Two distinct enums remain incompatible, even if
variants and payload layouts match. Existing payload arity/type checking and
numeric/array widening rules apply unchanged. Aliases to arrays, primitives or
records do not become enum constructor namespaces.

`CheckedEnumTypes::resolve_enum(name)` resolves canonical or validated flattened
alias names in that checked context. The earlier `find(name)` API retains exact
canonical-declaration lookup and still returns no alias declaration. This is
closed context resolution, not module/package visibility or filesystem authority.
The known type map and canonical-name index resolve aliases without walking
source alias chains again on every constructor/match lookup.

Callable checking retains generated alias signature spellings with the same
canonical constructor target IDs. Structural mapping validates the qualifier
against the checked environment before accepting the target. Runtime enum values,
formatting/equality, payload copying and model-v2 event charges remain canonical;
there is no alias wrapper, implicit conversion between distinct enums, new shared
state or additional runtime authority.

## Admission and compatibility

Every generated alias/variant signature consumes the existing per-function
signature count/text quotas and cumulative program signature-copy/text quotas.
Counts are checked before retaining/sorting alias spelling records; alias
signatures follow canonical constructors in sorted alias-name order. Exhaustion
refuses rather than removing aliases, skipping bodies or falling back to the
legacy checker. Even unused aliases contribute available signatures, as canonical
constructors already do.

The retained canonical enum-name index consumes one metadata node and the canonical
name's UTF-8 bytes per declaration in `EnumTypeLimits`. Existing metadata tests
include these extra retained records and still check exact/one-under refusal.
Alias targets reuse the already admitted flattened known-type map. These logical
compiler quotas do not establish hard time or physical-memory isolation.

Alias constructors inherit the existing E3110 standard-call collision refusal:
for example, an alias named `std` cannot expose `std::print` as a user enum
constructor. The diagnostic names the colliding callable and retains the original
variant location. This can refuse a v2 unit with an unused colliding alias; it
never changes standard-call identity or silently shadows a builtin.

Default legacy commands remain unchanged; the extended semantics require
`--experimental-v2`. Source spelling is preserved by the v2 formatter and IR
inspection retains canonical targets. CLI check/test/run, symbol projection and
source-backed native execution share the same checked identities. Native images
remain bounded source-backed envelopes rechecked at startup; they are not an
executable IR codec or stable cross-version ABI.

IDE variant references through aliases point to the canonical enum variant.
Alias declarations, annotation uses and qualified type prefixes still have no
navigation evidence. This does not add generic std::Option/std::Result, error
propagation, package linking or completion of G13.

See the [runnable example](../../examples/enum-aliases/README.md). The focused
`enum_alias_execution_v2` tests cover canonical equality, recursive calls, payload
widening, foreign/non-enum refusals, signature quotas and standard collisions.
Existing callable/type tests, CLI/formatter checks, real LSP/VS Code and actual
native qualification record their precise supported-host evidence separately.
