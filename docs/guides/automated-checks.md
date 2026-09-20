# Automated source checks

An agent or editor can request structured diagnostics:

```text
cargo run -- check --format json examples/functions/main.svr
cargo run -- check --format json tests/fixtures/untyped-parameter.svr
cargo run -- check --format json examples/fielddesk
```

The first command succeeds with no diagnostics. The second exits 1 with E3014
and the missing parameter annotation's location. The third checks only the
project's manifest and wiring; it does not establish that Fielddesk can run.

To inspect expression locations, check `tests/fixtures/expression-locations.svr`.
It exits 1: E3007 points to `42` in `std::len(42)`, and E3001 points to
`missing` in `print(missing)`. Byte offsets and character columns differ after
the fixture's Unicode text; use byte offsets when slicing UTF-8 source.

Check `tests/fixtures/project-locations` to inspect project scan locations.
Its malformed route and policy identify `main.svr` and `auth.svr` respectively.
Its missing handler/model and unsupported runtime also retain the reference or
assignment location. Project ranges cover the whole line; absent manifest keys
and filesystem failures can still have null locations. Resolve relative paths
against the command's working
directory, not against the project directory a second time.

For a built toolchain, invoke `svr check --format json <path>` and capture stdout,
stderr and the exit code separately. Cargo may write its own build progress to
stderr when using `cargo run`. Parse the report for check outcomes (0/1); for
usage errors (2), display stderr. Help requests return human text.

Use diagnostic codes to categorize issues and source ranges only when
`location` is non-null. After editing, run the check again; reports are snapshots
of that invocation, not persistent symbol identifiers. See the
[versioned schema](../reference/check-json.md) for offsets and limitations.
