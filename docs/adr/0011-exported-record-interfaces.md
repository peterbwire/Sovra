# ADR 0011: Exported record interfaces between packages

Status: Accepted option A (2026-10-03), explicitly approved by the user.
Implementation and verification remain required.

## Existing contract and missing decision

ADR 0009 approves nominal type identity and direct-import name resolution.
ADR 0010 implements exported functions from dependency entry modules. Local
records exist, but record declaration exports and cross-package field visibility
have not been selected. The compiler currently reports E4116 for imported record
signatures, preventing unrelated same-named records from being interchangeable.
Scalar aliases already resolve in the library and retain their existing behavior.

## Recommended option A: explicit record exports

Reuse the existing export modifier on a module's record declaration:

```svr
mod geometry {
    export struct Point {
        x: Float,
        y: Float,
    }

    export fn origin() -> Point {
        return Point { x: 0, y: 0 }
    }
}
```

This is proposed syntax, not yet executable. A consumer uses the existing module
import and keeps the dependency/module qualification for explicit type spellings:

```svr
use shapes::geometry;

fn magnitude_squared(point: shapes::geometry::Point) -> Float {
    return point.x * point.x + point.y * point.y
}

fn main() {
    let point = shapes::geometry::Point { x: 3, y: 4 }
    std::println(magnitude_squared(point))
}
```

1. A record is externally nameable and constructible only when explicitly
   exported from the imported module. Its fields are readable by consumers;
   all fields are part of its public interface for this milestone. Field mutation
   is not added. This rule does not preclude explicit field visibility or other
   encapsulation in a future milestone; such changes require a compatibility design.
2. An unexported record remains private. An exported function using a private
   record in its public signature is rejected. This first slice does not silently
   export private types, introduce opaque public handles, or add field-level access
   modifiers. Public record fields must also use externally accessible types.
3. Identity includes canonical local package identity, declaring module and
   declaration name. Two dependency aliases for the same package/module/record
   denote the same type; distinct packages or modules do not, even with identical
   field layouts. Source aliases never become nominal identities.
4. Resolve record signatures and field types in the declaring module's environment.
   Importing a module does not bring its types into a consumer's bare-name scope.
   Qualified annotations and constructor names use `alias::module::Record`.
5. Normalize scalar aliases as already implemented. Aliases to records preserve
   the underlying record identity and cannot expose a private record. This does
   not add exported alias declarations, generics, re-exports or recursive records.
6. Keep ordinary local struct syntax and source-stage APIs compatible where
   possible. Carry canonical type identities and resolved field metadata into
   semantic analysis and lowering; do not make nominal compatibility depend on
   runtime display strings or flatten unrelated records into one global map.

## Alternative B: opaque values through functions only

Permit record values to cross exported function boundaries through inference,
but expose no record annotations, constructors or fields to consumers. All access
goes through exported library functions. This avoids export syntax initially but
does not provide the requested field-metadata interface to ordinary consumers.
An explicit later decision would still be needed for public record declarations.

## Acceptance and compatibility

Test qualified construction, annotations, inferred factory returns, field access,
nested exported records, scalar-alias fields and Float widening in both engines.
Reject same-spelled records from other packages/modules, private type/field-type
leaks, missing/duplicate fields and wrong field types. Test two aliases for one
package, consumer name collisions, transitive visibility and source diagnostic
ranges. Validate every involved declaration before releasing linked IR.

Keep E4116 until each supported boundary is fully implemented; do not remove it
and return an unresolved type as a success. Document any remaining unsupported
recursive/type-export combinations. This completes neither M12 nor publication.

## Approval boundary

AGENTS.md requires: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." The approved nominal identity rule
does not by itself choose record export syntax or public-field visibility.
Approve option A or select option B before dependent implementation.
