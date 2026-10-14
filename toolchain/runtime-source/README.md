# Internal native linking source payload

This directory is a build-only snapshot of the trusted Rust implementation,
its embedded console template and the approved internal `native_builder`
example. It contains no checkout-specific absolute dependency path. Building
the example here binds its runtime dependency to this snapshot's location.

The developer machine needs compatible Rust/Cargo and a host linker. The
resulting application executable needs neither the source payload nor Rust,
Node or `svr` at runtime. This snapshot is not a separate runtime crate or a
stable cross-version ABI, and is not a full source/test distribution.

Internal proof invocation (from this directory):

```text
cargo run --offline --locked --example native_builder -- SOURCE OUTPUT_DIR
```

The builder checks the source/package and main entry, validates its private IR
image, refuses existing output, builds in staging and links against the runtime
source in this directory. Keep this payload at its original location while
using a builder compiled from it; rebuilding after relocation binds the new
location. Cargo and the linker are build-time dependencies.

`payload.json` records every source file's SHA-256. In the Windows toolchain
ZIP, the enclosing installation integrity manifest also covers this directory.
Hashes detect modification against the recorded bytes; verify the external
ZIP checksum when obtaining a package. Accepted ADR 0027 exposes the installed
`svr build --emit exe --output PATH SOURCE_OR_PACKAGE` command, which validates
the snapshot's complete `payload.sha256` list and compiler version before linking.
The hashes detect corruption against the recorded bytes, not publisher authenticity.
The compiler also retains check/run/IR/JS commands.
