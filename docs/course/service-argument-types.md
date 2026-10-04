# Checking service argument types

Status: Partial application checking. This lesson describes validation, not
service execution. Run `svr check --service-calls examples/service-contracts`.

Primitive service parameters accept compatible literals, known lexical bindings
and supported binary expressions. For a service operation `count(value: Int)`:

```svr
let count = 1 + 2;
mail.count(count);       // Int matches Int
mail.count(1 + 2.5);     // E4119: Float does not convert to Int
mail.count(true + 1);    // E4121: invalid operands
```

Int may widen to Float. A local declared `let amount: Float = 1` therefore retains
Float as its type. An incompatible known initializer produces E4120 instead of
supplying a misleading type to later calls. Nested bindings shadow outer bindings
only within their block, including when the inner type is unresolved.

String `+` produces String. Numeric arithmetic produces Int unless either operand
is Float. Numeric/String ordering and numeric/String/Bool equality produce Bool.
These checks do not evaluate expressions or prove absence of overflow or division
by zero. Resolved service call results contribute types when all arguments are
known compatible and the receiver resolves to one service. Local shadowing and
direct import visibility apply. Invalid or unknown calls do not supply types.
Same-file ordinary function results now resolve from primitive signatures,
including forward and recursive calls. Arguments must have known compatible
types and correct arity. Locals/parameters shadow ordinary names. E4127-E4131
cover duplicate declarations, call errors and ordinary return-contract errors.
See `examples/application-functions` for a checked example. Direct exported ordinary
imports are shown in `examples/application-imports`: `use app.helpers` exposes
`export fn format` as `app::helpers::format(...)`. Private helpers and transitive
imports stay inaccessible. Named application types and full implementation typing remain unfinished.
Complete syntax coverage does not mean complete type coverage.

The existing stdlib is also recognized: `std::len("hello")` contributes Int,
`std::to_string(12)` contributes String, and print/println contribute Unit.
Wrong arity or argument types fail ordinary call checks. An unknown argument is
still unresolved even when a builtin accepts Any; user-written Any annotations
are not wildcard types. E4132 rejects declarations colliding with builtin names.

Unresolved ordinary calls now produce E4133. This includes names without a supported
signature and calls through local bindings/function values. The JSON `ordinary_calls`
array shows resolution kinds and argument types; use its diagnostics and file
coverage too. A declared return type alone does not mean the call is valid.
E4134 rejects unresolved ordinary parameter annotations even in unused functions.
Use canonical primitive names; Any is not a user-defined wildcard annotation.

Known explicit returns inside service implementations are checked against their
contract with E4122. A bare `return` has Unit type; Int may widen to Float.
Non-Unit bodies that can fall through produce E4123. Both if/else arms must return;
a while loop alone never guarantees return. A return after the loop can satisfy
the rule. Known non-Bool conditions produce E4124. Unresolved conditions produce
E4125 and unresolved service returns produce E4126. These indicate that type
validation cannot finish, rather than a known mismatch. Branch and loop bindings
stay inside their blocks.
