# Local library example

`app` consumes `utilities` from a sibling directory. Both are real Sovra packages
using current executable syntax. No package installation or network access is used.

```text
cargo run -- check examples/local-packages/app
cargo run -- run examples/local-packages/app
cargo run -- build --emit js examples/local-packages/app
```

The run prints `49`. The library's exported square function uses a private helper.
See [the lesson](../../docs/course/local-packages.md) for visibility and limits.
