# Qualified standard Option and Result

The complete [generic source pipeline](generic-source-v2.md) now supplies two
compiler-owned nominal enums:

```svr
// Their fixed shapes; these are not user declarations of the std namespace.
// std::Option<T>: Some(T), None
// std::Result<T,E>: Ok(T), Err(E)
```

Use explicit type arguments for construction and qualified names for exhaustive
matching:

```svr
fn obtain<T>(value: T) -> std::Result<std::Option<T>,String> {
    return std::Result::<std::Option<T>,String>::Ok(std::Option::<T>::Some(value));
}
```

The [complete runnable example](../examples/generic-standard.svr) matches both
wrappers, checks the resulting value and exits silently:

```sh
svr check --experimental-generics docs/examples/generic-standard.svr
svr run --experimental-generics docs/examples/generic-standard.svr
svr build --experimental-generics --emit exe --output app docs/examples/generic-standard.svr
```

The same source works in a [guarded root package](generic-packages-v2.md), including
locked native builds. Qualified standard enum construction grants no host services.

`Option` and `Result` without `std::` remain ordinary user names. Standard enum
identity is an immutable compiler-owned tag on fixed synthetic declaration
fragments, registered only as `std::Option` and `std::Result`. User parsers cannot
set the tag or declare a qualified standard name. The short syntax name and
fragment spans retain the original template spelling; `qualified_name()` reports
the canonical registered name and `standard_identity()` exposes the tag. Standard
fragments never claim user-file provenance. Registry-owned nominal IDs keep user
and standard types distinct even when their variant shapes match.

Standard definitions are appended after user types. They consume existing
registry, parameter, definition, member, retained-source, annotation and checking
budgets even when unused. The two fixed templates have independent finite parser
profiles. Whole-file syntax usage and declaration listings continue to describe
user source only; generated standard definition costs belong to semantic-stage
admission. Low-level raw registration APIs remain explicit and do not silently
add a standard environment. No arbitrary module/type lookup is introduced.

Payload arity/types, exact generic argument identities, bounds, exhaustive arms,
immutable payload bindings and return-flow rules reuse the checked generic enum
pipeline. Different Option/Result instantiations are not implicitly converted.
Ordinary explicit payload numeric widening remains available at construction;
that does not convert an already wrapped Result<Int,E> into Result<Float,E>.
Nested wrappers and generic-function construction are checked symbolically and
rechecked after specialization. Arrays and other compound payloads preserve owned
snapshots. Some(Unit) is distinct from nullary None. Materialization charges the
qualified standard nominal label and variant label under existing VM fuel policy.

The standard-enum image profile uses feature word 3: base generic source bit 1
plus standard-enum bit 2. New images also include [standard Error](generic-error-v2.md)
under bit 4 and encode word 7. Decoding retains feature word 1 with its earlier environment,
so old images remain executable and cannot acquire standard enums by changing
only their embedded source. Word 2 alone and unknown bits refuse. The 16-byte
header, format version, source ceiling and all other admission remain unchanged;
legacy/plain-v2 image families remain separate. Newly encoded images require a
runtime with the advertised standard-enum/Error features. Source images recheck all bodies at startup.

`tests/generic_standard_v2.rs` covers constructors/matches, generic functions,
namespace/nominal separation, nested owned snapshots, strict invalid forms,
compiler-owned provenance, resource admission, Unit payloads and native feature
compatibility. `tests/generic_standard_cli_v2.rs` checks the exact documented
source and builds a real locked native package, then removes its source/manifest/
lock and executes with an empty PATH. Native/package execution qualification is
Linux only; other-platform and installed-runtime evidence remain separate.

Explicit adapters for existing specialized standard results,
postfix `?`, dependency/module linking, JavaScript/tooling integration and later
G13 libraries remain open. No existing builtin return type changes. This slice
does not complete G13 E2 or G13.

[std::Error](generic-error-v2.md) is now available as a separate qualified
nongeneric record; old word-3 images retain their earlier enum-only environment.
