# Explicit process hosts for experimental execution

```sh
svr run --experimental-v2 --allow-stdin main.svr
svr run --experimental-v2 --allow-files main.svr
svr run --experimental-v2 --allow-stdin --allow-files main.svr -- output.txt
```

These independent flags connect the existing v2 input/file interfaces to
`ProcessHost`. Both are absent by default. Only experimental `run` accepts them;
`check`, `test`, `build` and default commands do not gain new permissions.
Flags must precede the program-argument delimiter `--` and may appear only once.
After the delimiter they are ordinary arguments and grant nothing. Source code
cannot grant itself host access.

`--allow-stdin` grants bounded process line input through `RuntimeVmInput`.
The reader enforces strict UTF-8 and at most 1,048,576 payload bytes per line.
It removes LF and an immediately preceding CR; an unterminated final CR remains
part of the text. EOF is distinct from an empty line. Invalid UTF-8, oversized
lines and I/O errors refuse execution, without a fallback reader. Linked calls
share the same process stdin. The reader may consume bytes before refusing.

`--allow-files` grants the existing bounded UTF-8 `std::read_text`,
`std::write_text` and `std::write_text_if` operations through `RuntimeVmFiles`.
Paths resolve from the invoking process's working directory, under its OS
permissions. This includes absolute paths and accessible paths outside that
directory; the flag does not establish confined roots or separate read/write
permissions. Do not mistake it for a sandbox. Existing 16 MiB file ceilings,
structured error categories, atomic replacement and conditional-write
coordination contracts remain unchanged. Runtime materialization limits can
refuse a result even when the host's per-file limit admits it.

Whole-program preflight still runs before effects. A missing input/file grant
refuses even unused or unreachable calls requiring that capability. Granting one
does not grant the other. Parsing/checking/lowering failures have no service
effects. The [fixed runtime profile](experimental-v2-run.md) remains in effect;
there is no unlimited fallback or new host-policy override.

Print output stays captured until successful execution, even while stdin is
being read. Printed prompts are therefore not live interactive prompts. Blocking
host reads/writes cannot be interrupted by cooperative elapsed checks. Completed
service effects survive later arithmetic/fuel/materialization/control or output
publication failures; captured output can be lost while a file change persists.
These operations are not a transaction and the runner performs no rollback.

See the [enum text-copy fixture](../../examples/enum-text-copy/README.md) for a
program requiring both grants. This connects existing E1 host capabilities; it
does not complete the H1 confined filesystem/directory policy, general host
qualification, package/IDE integration or v2 native/JavaScript backends. Linux
acceptance is tested here; other platforms require their own qualification.

[Single-entry package selection](experimental-v2-packages.md) does not change
CWD, narrow file grants to a package root, or install any implicit hosts.
