# Node-free Sovra application execution

Status: **N1 partial proof implemented** (2026-10-12). This plan
changes the distribution goal: end users of a compiled Sovra application
should need neither Node.js nor Rust nor the Sovra compiler. The existing
interpreter, JavaScript backend, CLI, Task Manager CLI and browser GUI stay
available throughout migration. [ADR 0025](../adr/0025-standalone-executable-strategy.md)
records the approved first backend direction.

The internal `cargo run --example native_builder -- SOURCE OUTPUT_DIR` command
now compiles a checked `.svr` file or local package into a bounded private IR
image and builds a host-native release executable with the Rust interpreter.
The build machine needs Cargo; the resulting executable does not invoke Cargo,
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

## Verified starting point

The single Rust Cargo package exposes the compiler pipeline through `src/compiler`.
`svr run` parses and checks one `.svr` source or compiles a local dependency
package, lowers it to `IrProgram`, then calls `interpreter::run_with_host_status`
with `ProcessHost`. `ProcessHost` owns program arguments, UTF-8 stdin, flushed
stdout and the Rust text-file implementation. `svr build` emits human-readable
IR or Node CommonJS JavaScript. The JS backend imports `node:fs` and `node:path`
and uses Node process APIs; it is unsuitable as a Node-free application
artifact or direct browser module. The current `IrProgram` is a Rust data
structure, not a serialized portable artifact. There is no executable emitter,
standalone runtime crate, WebAssembly target or native UI renderer today.

The Task Manager CLI is a real `.svr` application: its direct and interactive
commands persist a versioned UTF-8 file through the general text APIs. Its
Node-hosted browser GUI invokes that same program and has a private Sovra UI
view/event prototype. Neither GUI route is a native window. The current
release candidate distributes `svr` plus Task Manager source; `svr` itself is
a native Rust binary, but the application still requires that compiler binary
at runtime. These distinctions matter for the clean-machine acceptance test.

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

The first `svr build --emit exe --target x86_64-pc-windows-msvc -o ...`
interface is illustrative, **not approved CLI syntax**. Build tooling would
compile Sovra source on the developer machine, serialize a bounded internal
IR image, generate a small Rust entry point embedding that image, and link it
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
