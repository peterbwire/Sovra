# Scalar aliases in application checking

Status: Implemented for the opt-in application checker. File-local record aliases
are now supported by [record contracts](application-record-contracts.md).
Exported aliases and application runtime execution remain unfinished.

Use the existing `type` declaration to name a scalar type:

```svr
type Amount = Scalar;
type Scalar = Float;

fn identity(value: Amount) -> Amount {
    let copy: Amount = value;
    return copy;
}
```

Declarations are file-scoped and may refer forward to other aliases. The final
target must be `Unit`, `Bool`, `Int`, `Float` or `String`. Aliases are transparent:
`Amount` above means `Float`, so an `Int` argument or initializer can widen to it.
This does not change the nominal identity of records in the executable compiler.

The same names work in service parameters/returns and typed task parameters.
Alias resolution precedes body inspection, so declaration order does not change
the meaning of a function interface. Local value bindings do not rename types.

Direct application imports expose exported functions, not alias names. An
exported function whose parameter uses a private scalar alias carries the resolved
scalar type into its consumer. A same-named alias in the consumer cannot change
that signature. No `export type` or qualified imported alias syntax is added.

Unknown targets and cycles produce existing executable diagnostic E3017;
duplicates and primitive-name collisions produce E3008. These errors retain
their declaring file and original byte ranges, even for unused aliases. An
invalid alias set supplies no validated alias types. Malformed declaration syntax
or unsupported nested aliases leave file inspection incomplete (E4096).

Try `svr check --service-calls --format=json examples/application-aliases`.
JSON call/local type evidence uses the canonical scalar name, such as `Float`.
This example is checked but is not an executable application runtime.
