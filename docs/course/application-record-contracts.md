# Application record contracts

Status: Partial application record support under `check --service-calls`.

The checker accepts file-local `struct` declarations using executable syntax:

```svr
type Scalar = Float;
struct Point { x: Scalar, y: Scalar }
type Position = Point;

service store {
    fn load() -> Point;
    fn save(value: Position);
}

fn identity(value: Position) -> Point { return value; }
fn main() { store.save(identity(store.load())); }
```

Record fields are validated even if no function uses the record. Unknown field
types produce E3017; duplicate fields or type declarations produce E3008. The
checker reuses executable declaration parsing and type validation.

Records retain nominal identity. A second record with the same fields has a
different type, as does a same-named record in another source file. Aliases to
one record preserve its identity. Function/service calls, return types and local
annotations compare these identities rather than comparing field layouts.

Application record names remain file-local. Service results can carry their
resolved identity through inferred locals in a caller. `export struct` now makes
record fields readable on values returned by a directly imported service, including
nested exported records. Private records remain opaque outside their declaring
file. E4116 rejects an exported field whose resolved record type is private, even
through an alias. This does not preclude future explicit field visibility.
Direct imports now expose exported record names in local annotations, ordinary
function signatures and constructors:

```svr
use app.shapes;
fn identity(value: app::shapes::Point) -> app::shapes::Point { return value; }
fn main() {
    let point: app::shapes::Point = app::shapes::Point { x: 1, y: 2 };
    identity(point);
}
```

Only directly imported `export struct` declarations are named this way. Private
records, aliases and transitive imports are not exposed; repeated imports do not
create a second nominal type. Service parameters and returns now also accept
directly imported record paths. A service in one module can return a record from
its direct dependency, retaining exported field metadata for its callers; this
does not expose that dependency's type name transitively. Private/transitive
annotation names fail with E4117. Direct imported records now also work as alias
targets and field declarations:

```svr
use app.shapes;
type Position = app::shapes::Point;
export struct Envelope { point: Position }
```

The alias keeps the imported record's identity. Its fields remain available through
nested exported records even when a caller imports only the wrapper module.
Names are still exposed only through direct imports. Invalid exporters provide no
usable type interface; E3017 remains for unresolved cross-file type-dependent
cycles. Exported ordinary functions may now accept and return exported records,
including via local aliases or direct imported types. For example, a factory can
return `app::shapes::Point`; its consumer may read the returned value's public
fields without directly importing `app.shapes`. This does not expose the source
type name transitively. Function interfaces carry declaring-file identity and
public field metadata. Exported ordinary functions that
expose private record types fail with E4116, including through aliases.

Canonical source-file paths currently identify application modules. These internal
identities are not portable published-package identifiers. Existing executable
package records and their approved export semantics are unchanged.

Field reads now work for records declared in the current file. For example,
`point.x` has type Float, and a nested `value.point.x` follows each validated field
declaration. These types propagate through locals, arguments and returns. Missing
fields produce E4138 at the member expression; invalid results do not propagate a
type. A field is not a callable method: `point.x()` produces E4133.

Construct local records with `Point { x: 1, y: 2 }`. Alias names may also be used
as constructors, and field values can contain nested constructors or calls.
Provide every field exactly once with a compatible value. Int values may widen
to Float fields. Unknown types, missing/extra/duplicate fields and incompatible
or unresolved values produce E4139; invalid constructors supply no nominal type.
Use parentheses around a constructor expression in a condition, for example
`if (Point { x: 1, y: 2 }.x > 0) {}`. Bare `if flag {}` remains supported.

Imported service values now carry exported field metadata with canonical identity.
Mutation and application runtime execution remain unfinished. Reads whose record
metadata is private or unavailable fail explicitly.
JSON scalar type fields retain null for nominal records, but additive `*_type_info`
descriptors now identify them with `kind: "record"` and an opaque `identity`.
Scalar field reads use `kind: "scalar"` with a canonical `name`; unresolved
descriptor values remain null. Compare identity tokens without parsing them.

null alone is not proof of an invalid type. Read diagnostics and inspection
coverage. Try the checker-only `examples/application-record-contracts` project.
