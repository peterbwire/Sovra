# Node-free Sovra application execution

Status: **N1 partial proof implemented**, updated 2026-10-14. This plan
changes the distribution goal: end users of a compiled Sovra application
should need neither Node.js nor Rust nor the Sovra compiler. The existing
interpreter, JavaScript backend, CLI, Task Manager CLI and browser GUI stay
available throughout migration. [ADR 0025](../adr/0025-standalone-executable-strategy.md)
records the approved first backend direction.

The internal `cargo run --example native_builder -- SOURCE OUTPUT_DIR` command
now compiles a checked `.svr` file or local package into a bounded private IR
image and builds a host-native release executable with the Rust interpreter.
The build machine needs Cargo and trusted runtime source; the resulting executable does not invoke Cargo,
Node or `svr`. This is an internal proof command, not public `svr build` syntax.
On Windows, for example:

```text
cargo run --example native_builder -- dogfood/01-task-manager/main.svr target/native-task-proof
target\native-task-proof\target\release\sovra-native-proof.exe add tasks.txt "Write tests"
```

The exact local Task Manager binary matched `svr run` for add, list, complete,
delete, exit status and state bytes across launches. Malformed-state and
missing-parent write failures also matched. Full release acceptance
still requires an isolated clean Windows image, import audit, broader failure
parity and reproducible packaging. The Node-hosted GUI is unchanged.

The internal `node scripts/test-native-task-manager.mjs` regression now builds
the proof executable in a temporary directory and compares 18 Task Manager
operations against `svr run`, including output, exit status, state bytes,
interactive stdin/EOF, legacy-file migration, corrupt-file rejection and a
write failure. On Windows it runs the proof with
a PATH limited to Windows system directories. CI runs this check on hosted
Windows; it is not clean-machine deployment evidence.

`scripts/build-native-task-manager.ps1` now uses public `svr build --emit exe`
and retains a Windows qualification ZIP
with `task-manager.exe`, a binary checksum, direct imports and a PowerShell
self-test. `build.json` records compiler version/hash, source hash and method.
The artifact test verifies the ZIP checksum, extracts it, validates provenance, checks
imports and runs the self-test; the 18-case parity runner then exercises the
extracted executable. CI uploads that exact ZIP and checksum for the later
[clean Windows VM test](../NATIVE_TASK_MANAGER_VM_TEST.md). This remains a
qualification artifact; clean-machine and hosted release evidence remain pending.

The Windows toolchain now carries an internal `runtime-source/` snapshot with
its own file hashes. The payload test relocates it, links Task Manager outside
the checkout, deletes runtime/source/build directories and runs all 18 parity
cases. This removes the checkout dependency for the internal proof; Cargo and
a host linker remain developer requirements. [ADR 0027](../adr/0027-native-build-cli.md)
approves the installed `build --emit exe --output PATH` command, now implemented
with payload validation, offline host linking and no-overwrite publication.

The internal `native_builder` example refuses an existing output path and
builds in a fresh sibling staging directory. It moves the complete Cargo
project and executable into the requested output only after a successful link;
a failed build removes its staging directory. Before staging, it requires a
parameterless `main` entry point and verifies that the generated private image
can be decoded. CI checks existing-output, missing-entry and missing-Cargo
failures without changing any public `svr` command.
The nested Cargo build uses an explicit staging target directory, so an
inherited `CARGO_TARGET_DIR` cannot redirect its executable. The default parity
runner exercises that environment setting; Linux and macOS CI also run this
host-native proof. Cross-compilation and inherited target selection remain
outside this host-native proof's supported scope.

`node scripts/test-native-packages.mjs` builds the existing local-package
example in a separate directory. It tests private helper calls, exported
nominal records, qualified signatures and Float widening against interpreter
output. After copying the executable, it deletes both the package source and
generated build project, then launches from two working directories with a
restricted PATH. CI runs this general package proof on all three host platforms;
hosted execution does not replace clean-machine deployment qualification.

`node scripts/test-native-runtime.mjs` builds one general-purpose fixture and
compares 22 cases against the interpreter after removing the generated source
copy and Cargo project. It checks overflow and retained output, division by
zero, array bounds, call-depth limits, exit code 125, missing/invalid/oversized
UTF-8 reads, the exact 16 MiB boundary, replacement bytes and failed writes
preserving existing files. The executable runs with restricted PATH. This
suite also exercises all 14 standard-library functions, the bare `print` alias,
array copy isolation, nominal equality, loops, short-circuit evaluation and
bounded stdin. It rejects missing fixture coverage when the builtin registry
changes and is included in all three host-platform CI jobs. See the
[capability matrix](../NATIVE_CAPABILITY_MATRIX.md) for evidence and remaining work.

## Verified starting point

The single Rust Cargo package exposes the compiler pipeline through `src/compiler`.
`svr run` parses and checks one `.svr` source or compiles a local dependency
package, lowers it to `IrProgram`, then calls `interpreter::run_with_host_status`
with `ProcessHost`. `ProcessHost` owns program arguments, UTF-8 stdin, flushed
stdout and the Rust text-file implementation. `svr build` emits human-readable
IR or Node CommonJS JavaScript. The JS backend imports `node:fs` and `node:path`
and uses Node process APIs; it is unsuitable as a Node-free application
artifact or direct browser module. The current `IrProgram` is a Rust data
structure. The internal native builder now serializes a private bounded image
and links a standalone runner; it does not establish a public portable format.
There is no separate runtime crate, WebAssembly target or native UI renderer today.

The Task Manager CLI is a real `.svr` application: its direct and interactive
commands persist a versioned UTF-8 file through the general text APIs. Its
Node-hosted browser GUI invokes that same program and has a private Sovra UI
view/event prototype. Neither GUI route is a native window. The current
release candidate distributes `svr` plus Task Manager source; `svr` itself is
a native Rust binary, but the application still requires that compiler binary
at runtime. The separate native proof ZIP embeds the runtime and program image.
These distinctions matter for the clean-machine acceptance test.

## Recommended first executable backend

The smallest credible path is **ahead-of-time validated IR embedded in a Rust
application runner**. This is a native Windows executable artifact, while its
Sovra instructions are initially executed by the existing Rust interpreter.
It is not yet native machine-code generation for each Sovra function.

```text
developer machine                              end-user machine
.svr or local package                         task-manager.exe
  -> parser / semantic / package linker          -> embedded validated IR
  -> checked IrProgram                           -> Rust runtime + ProcessHost
  -> versioned internal IR image                  -> application main(args)
  -> generated Rust runner + Cargo link          -> stdout/files/exit status
  -> task-manager.exe                           (no Node, Rust, or svr install)
```

The accepted first interface is `svr build --emit exe --output PATH SOURCE_OR_PACKAGE`.
It supports host builds; `--target` and cross-compilation are not implemented.
The build tooling compiles Sovra source on the developer machine, serializes a
bounded internal IR image, generates a small Rust entry point embedding it, and links it
with the runtime using Cargo/rustc. The Rust toolchain and Windows linker are
*build-time* dependencies only. The final `.exe` contains the program image and
runtime, not source files, a compiler process, a Node executable, or a JS
engine. An initial private, versioned image can change with the compiler;
published cross-version IR compatibility is not assumed. A later split of the
execution engine and host APIs into `sovra-runtime` can reduce build weight,
but the first proof can link the existing library and let the linker discard
unused compiler code. The image decoder must reject invalid lengths, opcodes,
jump targets, declarations and unsupported versions before execution; existing
source checking must remain mandatory. Do not use the current human-readable
`--emit ir` output as an executable format or embed `.svr` for runtime parsing.

The runner passes its own OS arguments directly to `ProcessHost`, calls
`run_with_host_status`, preserves Sovra's 0–125 process-exit contract and
prints runtime failures to stderr with a nonzero exit. It uses the process
working directory and the existing `std::read_text`/`std::write_text`
semantics. No new language-visible string, array, record or memory behavior
is required for this first milestone. The runtime must retain current value
copy/equality, numeric overflow, call-depth, UTF-8 and 16 MiB file limits.
Any change to those contracts needs separate approval and parity tests.

The immediate proof is a standalone `task-manager.exe` that performs add,
list, complete and delete against one state file across independent launches.
Test the exact executable on a clean compatible Windows VM with Node.js,
Rust/Cargo and `svr` absent; assert stdout, stderr, exit codes and file bytes.
Inspect imported DLLs and document any system or C runtime dependency. Rust
supports the Windows MSVC target, but a build does not itself prove deployment
independence. If static C runtime linkage is selected, inspect the resulting
binary and test it; Rust's `crt-static` option is not a substitute for that
check. Supported Windows version and architecture must be stated before
release. See the [Rust MSVC target guide](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
and [Rust linkage reference](https://doc.rust-lang.org/reference/linkage.html).

## Native code generation choices after the first proof

| Path | What it provides | Cost and first-use decision |
| --- | --- | --- |
| Embedded Rust IR runner **recommended first** | Standalone PE executable, semantic reuse, minimal backend risk | Application instructions remain interpreted; binary may be larger and startup/runtime slower. Prove distribution and runtime parity first. |
| Cranelift object backend | Direct native machine code/object output from lowered functions | Requires value ABI, calls, strings, arrays, records, errors, host calls, ownership and linker integration. `cranelift_object::ObjectModule` emits object files, so a linker and runtime remain necessary. Consider after the IR runner's host ABI is stable. |
| LLVM backend | Mature target and optimization infrastructure, PE/COFF on Windows | Broad capability but a larger toolchain/integration burden; the same runtime/value ABI decisions still apply. Consider when optimization or platform coverage justifies it. |
| Transpile to Rust then compile | Native code using Rust's compiler and standard library | Faster experiment for a limited source subset, but generated Rust must preserve Sovra value, error and overflow semantics exactly; it risks making Rust syntax the accidental language specification. Not the recommended first backend. |

Cranelift's object API and LLVM's Windows COFF support are documented by
[Cranelift](https://docs.rs/cranelift-object/latest/cranelift_object/struct.ObjectModule.html)
and [LLVM](https://llvm.org/docs/GettingStartedVS.html). Neither removes the
need for a Sovra runtime or a stable host-call boundary. Do not add either as
a dependency merely to package the first Task Manager executable.

## Runtime and UI boundaries

The initial native runtime reuses the Rust `Value` model: `Int`, `Float`,
`Bool`, UTF-8 `String`, value-semantic arrays and nominal records, plus `Unit`.
It must support function frames, checked operations, output, arguments,
line input, text-file results and process status. Errors keep their current
source/runtime categories; the runner must not panic on untrusted application
files. Native machine-code work later needs an explicit value representation,
allocation and lifetime plan, deterministic cleanup, bounds and host function
ABI. Those are **not** chosen by the executable-container milestone.

The eventual unified UI should preserve the architecture in
[SOVRA_UI.md](SOVRA_UI.md): Sovra owns state, events and domain rules; a host
provides platform capabilities; renderers map a reviewed, accessible view
contract to platform widgets. First, keep the working Node browser GUI as a
regression baseline. A Node-free local browser host could later move HTTP and
HTML serving into Rust, but the current private `SVR-UI-1` stream is not a
public UI ABI. A native desktop host could use a system webview or a native
widget/graphics renderer. A webview avoids Node but may require a separately
installed WebView2 runtime on Windows; deployment must detect or bundle that
dependency. A pure native renderer avoids that browser runtime but requires
layout, text, input and accessibility work. Wry/WebView2 and eframe/egui are
options to evaluate with prototype evidence, not selected dependencies.
See [Microsoft's WebView2 distribution guide](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution),
[Wry](https://docs.rs/wry/latest/wry/) and [eframe](https://docs.rs/eframe/latest/eframe/).
Desktop, Android and iOS adapter contracts require separate approval. A CLI
executable must not wait for the UI framework.

A browser WebAssembly target is a later backend, not a wrapper around the Node
CommonJS output. It should consume checked IR and implement only explicitly
granted browser capabilities through host imports; filesystem/process APIs
cannot be assumed. Strings, collections, memory, async events and UI
transports need an approved ABI. Start with a pure computation fixture, then
browser event/view parity, and only then application migration. The browser
must be tested without Node; generated JS glue, if used, is a browser asset
and not a Node installation requirement. See the
[Rust WebAssembly target documentation](https://doc.rust-lang.org/rustc/targets/index.html)
and [MDN's WebAssembly loading guide](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Loading_and_running).

## Incremental milestones and gates

These are **new work packages, not replacements or renumberings of M0–M15**.

1. **N1 — Internal IR image and standalone CLI proof.** Specify a private
   versioned bounded image, code generator and Rust runner. Compile a small
   fixture and Task Manager. Match interpreter/JS outputs, errors and exit
   codes. On clean Windows, run the exact `.exe` without toolchains or Node.
2. **N2 — Distribution-grade runtime split.** Isolate execution and host
   services from parser/compiler dependencies; audit generated binary size,
   imports, startup, crashes and supported OS/CPU. Add reproducible target
   builds, checksums and install/uninstall/update guidance. Keep existing
   `svr run`, `--emit ir` and `--emit js` behavior and tests.
3. **N3 — Native function code generation, if justified.** After value/host
   ABI approval, prototype Cranelift on pure scalar functions, then strings,
   records, arrays and stdlib calls. Differential-test every step against the
   interpreter and JS backend. LLVM remains an alternative if requirements
   justify its cost.
4. **N4 — Node-free desktop host.** Keep Task Manager's business logic and
   state format; replace only the GUI host/renderer behind an approved UI
   contract. Test add/complete/delete, accessibility and restart persistence
   on clean Windows. The Node GUI remains available during migration.
5. **N5 — Other platforms and browser WebAssembly.** Build native Linux/macOS
   artifacts on their supported runners. Plan Android/iOS signing, packaging
   and lifecycle separately. Bring up Wasm with pure computation before any
   browser UI or storage claim.

CI should build each supported target on its native runner, test the current
compiler and dogfood suite, run standalone fixture/Task Manager binaries,
inspect runtime imports, archive exact bytes with checksums, and retest the
downloaded artifact. Windows must include a clean-machine test with no Node,
Rust or `svr`; PATH filtering alone is weaker evidence. Cross-compilation is
not assumed for MSVC targets. Linux and macOS packages need their own dynamic
library and minimum-OS audits. Signing/notarization and installers follow
once target support and update policy are approved.

## Approval and current stopping point

N1's internal executable proof is implemented. No public CLI syntax or
cross-version IR contract is approved. Windows target/CRT policy, native
value/ownership ABI, public UI contract, browser capability model and mobile
targets remain separate decisions. The private image is bounded to 32 MiB and
one million entries per list; these are prototype artifact limits, not Sovra
source-language semantics.
