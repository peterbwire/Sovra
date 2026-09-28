# Composing library-style inline modules

Implemented scope: one executable source file. Separate package consumption is
planned, not demonstrated by this lesson.

Run the [example](../../examples/library-foundations/main.svr):

```text
cargo run -- check examples/library-foundations/main.svr
cargo run -- run examples/library-foundations/main.svr
cargo run -- build --emit js examples/library-foundations/main.svr
```

The run prints:

```text
square: 49
letters: 5
```

`arithmetic::square` calls its private `arithmetic::product` helper. The consumer
passes the result to `std::to_string` and `strings::label`. That exported string
function calls its own private separator helper. Calling either private helper
from `main` fails source validation. Explicit parameter annotations define these
interfaces; return annotations and bodies receive ordinary semantic checks.

This is useful code organization with tested execution, not independently
installable libraries. There is no dependency manifest syntax to teach yet.
The example adds neither standard-library builtins nor compiler special cases.
