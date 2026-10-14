# ADR 0027: First public native build command

Status: **Accepted (2026-10-14).** The user approved the proposed first-slice
contract. Implementation and qualification evidence are tracked in the
development log and native capability matrix.

## Context

Accepted ADR 0025 approves embedded checked IR with the trusted Rust runtime,
but leaves public native build syntax and deployment policy to a separate
decision. Internal tests now cover Task Manager, local packages and all current
builtins. A relocated runtime source payload builds Task Manager without the
Sovra checkout. The Windows toolchain packages that snapshot.

The next step is an installed compiler command invoking this same general
pipeline. It must not change existing IR/JS output or language semantics.

## Accepted contract

```text
svr build --emit exe --output PATH SOURCE_OR_PACKAGE
```

- `--emit exe` builds a host-native executable using checked source/local
  package IR and the embedded Rust interpreter. It does not promise native
  machine-code generation for each Sovra function.
- `--output PATH` is required for executable builds and names the final file.
  Windows paths must end in `.exe`; other host paths have no required suffix.
  Explicit output avoids changing stdout/default artifact assumptions.
- No `--target` or cross-compilation support in this first slice. The installed
  runtime snapshot, compiler and Rust/linker host must be compatible.
- Locate `runtime-source` beside the installed `bin` directory. Development
  builds may use the matching repository source; tests must cover both cases.
  Validate the payload's complete file list, hashes and compiler version before
  invoking its build tools. Missing/mismatched/damaged payloads fail clearly.
- Rust/Cargo and a compatible linker are developer build-time requirements.
  Document them and report missing tools; do not download/install them implicitly.
  Programs contain their runtime and checked program image at completion.
- Refuse an existing output file, directory or link. Build in an owned staging
  directory on the destination filesystem, publish only after successful linking,
  and enforce no-overwrite behavior at publication time. Failure leaves any
  existing file untouched and cleans owned staging. Parent directories may be
  created; unrelated files are never removed.
- Exit 0 on success, 1 for source/payload/build errors and 2 for usage errors,
  consistent with existing CLI categories. Print the final path on success;
  compiler/build diagnostics go to stderr. Retain structured source checking
  through `svr check --format json`; no new native JSON protocol in this slice.
- `build` without `--emit` and `--emit ir|js` retain current stdout behavior.
  `--output` is initially accepted only with `--emit exe`; unsupported combinations
  are usage errors rather than silently changing existing output rules.

## Acceptance before marking implemented

Test valid/invalid command combinations, existing-output refusal and publication
races, malformed source/packages, missing Cargo/linker, corrupted/missing/version
mismatched payloads and staging cleanup. From an installed toolchain outside the
checkout, build the console template, Task Manager and a local package consumer.
Remove source/runtime/build dependencies and verify exact executable output,
errors, status and files. Retain all interpreter/JS and installation tests.

Clean compatible Windows VM deployment and supported-host qualification remain
required before publishing a supported executable target. This command does not
add a UI renderer, registry, new memory model, public IR format or ABI.

## Approval

The user approved the invocation, required output/no-overwrite policy, initial host-only
target scope, runtime discovery/validation and build-time Rust/linker requirements
as one first-slice contract. AGENTS.md requires pausing before compatibility
decisions. This contract is approved; subsequent target/ABI changes require
their own review.
