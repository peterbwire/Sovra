# Generic programs as host-native executables

```sh
svr build --experimental-generics --emit exe --output app docs/examples/generic-source.svr
./app
```

On Windows choose `--output app.exe`. Rust/Cargo and a compatible host linker are
build-time requirements. The resulting executable contains the Rust VM and
embedded Sovra source; it needs no Node.js, Rust, Cargo, compiler installation or
original `.svr` file at runtime. This is host-native packaging with checked VM
execution, not direct machine-code lowering of generic Sovra operations or a
cross-compilation claim.

The explicit source/root-package build mode uses the [generic source pipeline](generic-source-v2.md),
requires parameterless Unit `main` and selects no language host services. Like
[generic CLI run](generic-cli-v2.md), native execution uses finite instruction,
materialization, work, frame and active-storage limits, zero output bytes and a
30-second cooperative elapsed limit. These controls are not a process sandbox
or hard CPU/heap deadline. Native invocations reject any supplied program argument;
there is no implicit stdin, filesystem, network or output authority.

`CompiledGenericSource::encode_generic_native_image` creates a private source
image. `decode_generic_native_image` validates it and repeats complete bounded
source compilation, including unused bodies and concrete IR validation.
`execute_generic_native_image` decodes/rechecks and executes under the fixed
`generic_command_runtime_policy`. `validate_command_entry` is shared by CLI run,
build and image admission. Noncommand library roots and non-Unit main functions
cannot be encoded.

The 16-byte header contains six-byte magic `SVRG2\0`, format version 1, value
model 2, feature word 7 and little-endian u32 UTF-8 source length.
[Standard enums](generic-standard-v2.md) use bit 2 and
[standard Error](generic-error-v2.md) uses bit 4, alongside base bit 1. Old word-1
images retain their standard-free profile; word 3 retains Option/Result only. Source is
limited independently to 1 MiB. Short, oversized, trailing, mismatched-length,
invalid UTF-8, unknown version/model/feature and malformed or invalid source
images refuse before execution. The family detector is only a dispatch hint.
The header is not authentication and this private format is not a stable ABI.
No source pathname, compiler quotas, permission grants or environment are encoded.

The existing native linker validates the source image before linking and uses
validated runtime-source discovery, staged offline Cargo compilation and atomic
no-overwrite publication. Existing output, including links, is refused. A failed
source/entry/image check cannot publish an executable. The shared runner has a
separate dispatch branch for generic images. Legacy IR images and plain v2
source images keep their prior formats and execution paths; neither decoder
accepts generic images. Native errors are escaped and capped before stderr
publication, with embedded source coordinates where available.

Build accepts one standalone `.svr` input or a
[guarded root package](generic-packages-v2.md), optional package-only `--locked`,
and exactly one `--emit exe` plus
`--output PATH`. Unknown/duplicate options, IR/JS output, permissions, other
engine modes and program arguments refuse. Dependency/module linking,
JavaScript, generic test/tooling integration and structured standard errors
remain open. This slice does not complete G13 E2 or G13.

`tests/generic_native_image_v2.rs` checks exact admission boundaries, every image
prefix, header corruption, semantic tampering, entry rules, fixed policy, runtime
refusal and old-format separation. `tests/generic_native_cli_v2.rs` builds a real
application using the documented generic example, removes its source, executes
with an empty PATH, checks argument refusal and staging cleanup, and verifies
strict flags and output preservation. Qualification recorded here is Linux;
Windows/macOS installation and execution still require actual-host evidence.

Current propagation integration: [experimental command/native support](generic-propagation-cli-v2.md)
now enables the explicit source profile under `--experimental-generics` and native
feature word 15. Historical profiles 1/3/7 and ordinary source APIs remain strict.

Adapter environment update: [explicit standard result adapters](generic-result-adapters-v2.md)
now extend experimental commands and select native feature word 31. Historical
words 1/3/7/15 retain their previous environments. Host builtin calls remain gated.

Host-free builtin update: [checked parsing and splitting](generic-builtin-calls-v2.md)
now use the explicit builtin source profile and experimental CLI/native feature 63.
Schema-only profile 31 stays strict; other host/builtin operations remain gated.

String utility update: [checked String calls](generic-string-builtins-v2.md)
now extend experimental commands and native feature 127. Earlier source APIs and
image words retain their previous environments; host I/O remains gated.

Value utility update: [display and array length](generic-value-builtins-v2.md)
now extend experimental commands and native feature 255. Earlier APIs/image
environments retain their previous scope; host I/O remains gated.

Array operation update: [growth and slicing](generic-array-builtins-v2.md)
now extend experimental commands and native feature 511. Earlier APIs/image
environments retain their previous scope; host I/O remains gated.
