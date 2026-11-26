# Experimental data and host profile

This opt-in package profile adds bytes, strict JSON and explicit host requests to
the Rust generic VM. Legacy commands retain their existing behavior. Task Manager
and UI behavior are outside this change.

## Commands and native execution

[Format generic/data source](generic-formatting.md) with `svr fmt --experimental-generics`
(or add `--package --write` / `--package --check` for a package graph).

For a package directory, use both `--experimental-generics` and
`--experimental-data` with `check`, `run`, `test` or `build`. Generate and verify
the package lock as for other experimental generic packages. For example:

```sh
svr check --experimental-generics --experimental-data --locked ./app
svr run --experimental-generics --experimental-data --locked ./app
svr build --experimental-generics --experimental-data --locked --emit exe --output ./program ./app
```

The exact native image profile word is 2047. Older profiles refuse data images;
there is no best-effort decoding. The generated executable embeds checked source
and a Rust runtime: it executes without Node.js or the original project files.
This is source-backed VM execution inside a native executable, not machine-code
optimization of each Sovra function. Independent launch is tested on Linux.

Host authority is absent by default. Only `run` accepts `--host-policy PATH`;
the native executable accepts the same optional pair. Version-three policies select file/process/time services; [version-four network
policies](network-policy-v4.md) separately select explicitly granted loopback
networking. The policy remains an
external trusted file and is never embedded in the image or accepted from an
environment variable. Check/test/build cannot acquire grants through this flag.
Standalone files, JavaScript/worker execution and editor/LSP data-profile options
are unsupported; editor/LSP requests for these options explicitly refuse them.

## JSON

`std::JsonValue` has `Null`, `Bool(Bool)`, `Int(Int)`, `Float(Float)`,
`String(String)`, `Array([std::JsonValue])` and
`Object(std::Map<String,std::JsonValue>)` variants. The explicit parse/stringify
builtins return sealed standard Results; there is no implicit record conversion.
See the executable [data consumer](../../examples/data-toolchain/app/main.svr).

Integer syntax must fit signed 64 bits. Fractions/exponents produce finite floats;
serialization preserves numeric kind. Objects retain insertion order and reject
duplicate decoded keys, including escaped spellings of an existing key. Unicode,
surrogates, escapes, number grammar and UTF-8 are checked strictly. Error offsets
refer to original zero-based UTF-8 byte positions. Serialization is not a
canonical format for signing.

Default JSON ceilings are 16 MiB input/output, depth 128, one million combined
values/members, 64 MiB text and 128 million work units. Trusted Rust callers can
lower these ceilings. VM instructions, owned nodes/text, control and shared work
also apply. Temporary JSON trees and output staging have their own bounded
budgets; these are not a claim that every allocator byte is charged to VM fuel.

## Version 3 host requests

`std::host_call(request)` takes a `std::JsonValue` object and returns
`std::Result<std::JsonValue,std::Error>`. Every request contains `"version":3`
and `"op"`; unknown fields refuse. Paths are relative to a named granted root.
Binary request/reply fields use strict Base64.

| Operation | Additional fields |
| --- | --- |
| `path.normalize`, `path.basename` | `path` |
| `path.join` | `left`, `right` |
| `date.parse` | `text` |
| `date.format` | `milliseconds`, `offset_minutes` |
| `date.add` | `milliseconds`, `delta` |
| `clock.wall`, `clock.monotonic` | none |
| `timer.sleep` | `milliseconds` |
| `fs.read`, `fs.list`, `fs.metadata`, `fs.mkdir`, `fs.remove` | `root`, `path` |
| `fs.write` | `root`, `path`, `data` |
| `fs.rename` | `root`, `path`, `to` |
| `process.run` | `program`, `arguments`, `input` |

Paths/dates are pure. Clocks, timers, filesystem and processes need explicit
grants. Dates use Gregorian years 1–9999 and UTC or fixed offsets with millisecond
precision; leap seconds, named zones and local-zone guessing refuse. Wall clocks
are not deadline clocks.

A trusted policy is bounded to 64 KiB, for example:

```json
{"version":3,"read_roots":{"assets":"/srv/assets"},"write_roots":{"output":"/srv/output"},"clocks":false,"timers":false,"processes":{"copy":{"executable":"/usr/bin/cat","cwd":"/srv/work","environment":{}}},"limits":{"bytes":1048576,"entries":100,"arguments":10,"argument_bytes":4096,"milliseconds":1000}}
```

All limits can only narrow the defaults: 16 MiB per file/stream, 4096 directory
entries, 1024 arguments, 64 KiB combined argument/environment text and 30 seconds
per child/timer. Policy files contain authority and must come from a trusted
operator, not from untrusted Sovra source.

## Ownership and supported host boundary

Filesystem/process adapters currently require Linux procfs and x86, x86_64 or
aarch64. Other hosts refuse unsupported operations. Windows cross-compilation
checks establish build compatibility only, not Windows runtime qualification.
Captured directory handles and no-follow component traversal guard roots and
symlinks. Read/write grants are separate. This is a capability API, not an OS
sandbox: external directory mutation and hard links require a trusted deployment
policy. Earlier filesystem effects can remain after failure or cancellation.

Processes use an explicitly granted native ELF executable, fixed cwd and explicit
environment, with no shell or inherited environment. The executable receives
ordinary OS process authority; filesystem grants do not sandbox that executable.
The adapter owns child/stdin/stdout/stderr, bounds streams and deadlines, and
kills and reaps its child before reporting successful cleanup. Cleanup failures
surface explicitly. Trusted embedders must not independently reap adapter-owned
children. Descendant termination and hard CPU/memory isolation are not promised.

## Reproducible evidence

Run `python scripts/qualify-g13.py --all-rust --node` from the repository with
stable Rust, exact Rust 1.74.0 and Node installed. Native/process tests need an
environment that permits executing local binaries through procfs. The harness
retains per-command logs and a source-fingerprinted report under
`target/checked-evidence/g13-closure/`. G13 closure still depends on passing the
consolidated qualification; implementation alone does not close Q1.
Optional real VS Code Electron acceptance uses `--vscode-host` and requires its
separately provisioned dependency and display/headless environment; omission is
recorded explicitly in the report.
