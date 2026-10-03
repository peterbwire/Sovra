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
Ordinary function results, named application types and full implementation typing are
still unfinished. Complete syntax coverage does not mean complete type coverage.

Known explicit returns inside service implementations are checked against their
contract with E4122. A bare `return` has Unit type; Int may widen to Float.
Non-Unit bodies that can fall through produce E4123. Both if/else arms must return;
a while loop alone never guarantees return. A return after the loop can satisfy
the rule. Known non-Bool conditions produce E4124. Unknown condition and return
expression types remain unchecked. Branch and loop bindings stay inside their blocks.
