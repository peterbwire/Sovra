# Experimental v2 source-backed native executables

```sh
svr build --experimental-v2 --emit exe --output app main.svr
svr build --experimental-v2 --locked --emit exe --output app package-directory
```

This builds a host executable containing the Rust compiler/VM and a private v2
source image. Rust/Cargo and a host linker are build-time requirements. The
executable requires no Node.js, `svr`, Rust toolchain, original source files or
runtime-source directory at execution time. It is a source-backed interpreter
executable: the bounded frontend checks the embedded source and lowers it at
startup. It is not an optimizing backend, an executable IR codec, cross-compilation
or a stable binary ABI. Source remains embedded in the binary; startup includes
compiler work. JS output remains unsupported for v2.

## Checking and publication

Build checks all functions, requires a parameterless Unit `main` and validates
linked lowering before invoking the existing native builder. Locked packages use
the same captured entry/manifest hashes and strict lock checks as other v2
commands. Dependency/registered-module compilation remains unavailable. An
existing destination refuses; staged linking publishes without overwriting a
concurrently created destination. Runtime-source discovery, installed-payload
hash/file admission and staging cleanup reuse the existing builder. A failed
cleanup after publication is reported; build-time Rust/Cargo is trusted tooling,
not a sandbox. Native compilation uses the existing builder's process behavior.

The new private image family is `SVRV2`, with an exact format version, value-model
version, supported-feature mask and length-prefixed UTF-8 source. The current
format carries only source, at most 1 MiB. No host grants, original source path,
compiler flags or environment are serialized. The decoder rejects unsupported
versions/models/features, malformed UTF-8, length mismatches, trailing bytes and
oversized/truncated images before checking. It rechecks every body under the
finite default source limits, and entry/linked lowering must succeed before
execution. The header is representation admission, not authentication: a
semantically valid source edit is not detected as tampering by this codec.
Application authenticity requires the distribution's separate trust mechanism.

Legacy `SVRIR` images retain their existing codec/execution path. Neither decoder
accepts the other representation. The runner dispatches v2 images explicitly;
malformed v2 headers fail rather than falling back to a different compiler.

## Execution authority and limits

Arguments are application data; neither source nor an argument can grant file or
stdin services. This initial v2 runner supplies no such services, including for
read_line/read_text/write_text/write_text_if. Build grant flags are usage errors.
The VM preflights standard calls across the linked program: even an unused
file/input call refuses the invocation without a grant. The runner does not
inherit legacy native file/input authority. Privileged native
services remain a separate integration gate; OS permissions still apply to the
trusted Rust runner itself.

The runtime profile is shared with the experimental CLI: one million instructions,
1 MiB captured output, one million cumulative value nodes, 16 MiB cumulative text,
256 frames and a cooperative 30-second execution deadline. Compiler/lowering work
precedes the execution clock and has its own logical quotas. These are not hard
process time/memory isolation. Native arguments are additionally bounded to 1024
entries and 64 KiB aggregate UTF-8 bytes, without lossy Unicode replacement.

Successful completion publishes captured output and the portable 0..125 program
status. Runtime refusal returns status 1 without publishing pending output.
Output OS write failures can leave a partial write and return 1. Compilation and
link failures return 1; malformed/unsupported build options return 2. `--output`
is required for `--emit exe` and unavailable for IR inspection.

## Qualification and remaining gates

`tests/native_image_v2.rs` covers exact/exceeded admission, malformed images,
semantic rechecking, old/new representation separation, enum calls, arguments,
status, finite fuel and denied services. `scripts/test-native-v2.mjs` builds a
locked package, removes its source and executes a relocated binary with tools
removed from PATH; it also checks no-overwrite publication, stale-lock refusal,
denied I/O, failure output and staging cleanup. Host support is claimed only when
that actual native acceptance passes on the target. Installed/Windows/macOS native
v2 qualification remains separate from source-build Linux evidence.

G13 remains partial. An executable IR codec, v2 module/dependency linking and
symbol evidence, generics/Option/Result/propagation, Map/Set, JSON/codecs and the
approved filesystem/time/process capabilities still need their own implementation
and qualification. A working Node-free executable alone does not close them.
