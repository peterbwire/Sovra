# Examples: choose the right capability

Run the commands below from the repository root after `cargo build --locked`.
With an installed compiler replace `cargo run --` with `svr`; paths still refer
to the example tree in your checkout or installed package. The
[machine-readable catalog](catalog.json) classifies every example directory;
`node scripts/test-example-catalog.mjs` verifies the classifications and commands.

## Executable language examples

These use implemented executable syntax. Each can be checked, run through the
Rust interpreter, and built/executed through the Node JavaScript backend.

| Example | Entry source |
| --- | --- |
| Greeting | [hello-world](hello-world/main.svr) |
| Typed functions | [functions](functions/main.svr) |
| Numeric rules | [numbers](numbers/main.svr) |
| Strings | [strings](strings/main.svr) |
| Nominal records | [records](records/main.svr) |
| Inline modules | [modules](modules/main.svr) |
| Scoped types | [scoped types](scoped-types/main.svr) |
| Library-style composition | [library foundations](library-foundations/main.svr) |

For example:

```sh
cargo run -- check examples/functions/main.svr
cargo run -- run examples/functions/main.svr
cargo run -- build --emit js examples/functions/main.svr
```

Build prints JavaScript; running that output requires Node. This is not a claim
of browser execution or native GUI qualification.

[Local packages](local-packages/README.md) are executable too: check/run/build
`examples/local-packages/app`. Its sibling library is consumed through the local
dependency graph; package installation/registry access is not required.

[Snapshot slicing packages](array-slice-packages/README.md) provide another
executable local library/consumer, including typed tails, nominal record arrays
and independent snapshots. This requires the current G13 slice builtin.

## Executable host component

[UI desktop component](ui-desktop/README.md) is real Sovra code that emits views
and handles explicit events. It needs its documented command arguments and a
browser/desktop host for an interactive window. View generation alone does not
open a native UI:

```sh
cargo run -- run examples/ui-desktop/component.svr -- ui-layout-view target/example-message.txt
```

Native WPF interaction requires Windows and its separate acceptance suite.

## Checker-only application examples

These demonstrate supported static application inspection. They do not start
services, pages, scheduled work or an application runtime:

- [Application aliases](application-aliases/README.md)
- [Application functions](application-functions/README.md)
- [Application imports](application-imports/README.md)
- [Record contracts](application-record-contracts/README.md)
- [Imported record contracts](application-record-imports/README.md)
- [Service contracts](service-contracts/README.md)

```sh
cargo run -- check --service-calls --format json examples/application-functions
```

## Proposed application syntax

[Fielddesk](fielddesk/README.md) describes the future integrated application
surface. `check examples/fielddesk` checks its manifest/wiring; success does not
make its bodies, authentication, routes or UI executable. Its proposed `run` and
`test` flow is not a runnable tutorial. Start with the executable examples above.

## Internal Rust tools

`function_probe.rs` and `native_builder.rs` are Rust development helpers, not
Sovra application examples or public CLI commands. See
[developer tools](../docs/DEVELOPER_TOOLS.md) and
[native execution architecture](../docs/architecture/NODE_FREE_APPLICATIONS.md).


The [array growth consumer](array-slice-packages/growth-app/README.md) independently
uses the registered collection library for filtering, numeric concatenation and
nominal record results. It is executable on the current G13 compiler, not a planned
registry or browser application.
