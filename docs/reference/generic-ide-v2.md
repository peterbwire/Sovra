# Experimental generic editor mode

Enable `"sovra.experimentalGenerics": true` in VS Code user settings and run
**Sovra: Restart Language Server**. Set `sovra.experimentalV2` to false; enabling
both modes refuses initialization. The default legacy and existing v2 modes remain
separate. A current compiler supporting `--experimental-generics` is required.

This mode checks unsaved standalone buffers with the compiler's complete generic
source pipeline, including generic functions/types, Option/Result/Error, explicit
adapters, `?` and supported host-free builtins. Checking does not execute source.
Original byte ranges are converted to UTF-16 editor ranges, including Unicode and
CRLF. Existing buffer/job/debounce/generation/cancellation limits and temporary
file cleanup remain active (512 KiB per buffer, 4 MiB total, 32 open documents).

Check and Run source tasks save and validate the selected file using the existing
task preparation protocol, then pass `--experimental-generics`. They require a
trusted workspace. Even inside a package, diagnostics/tasks select a standalone
file by default. [Explicit generic package tasks](generic-package-tasks.md) now
select compiler-owned graph sources when `sovra.experimentalGenericPackageTasks`
is enabled. [Unsaved generic package diagnostics](generic-package-diagnostics.md)
are now a separate opt-in through `sovra.experimentalGenericPackageDiagnostics`.
[Package function navigation](generic-package-navigation.md) now follows checked
imports with source fingerprints. [Package type annotation navigation](generic-package-type-navigation.md) follows
checked imports and aliases; body type/field/local package references remain open;
unavailable facts clear rather than reuse standalone guesses.
Run retains the CLI's parameterless Unit main and bounded host-free policy.
Native builds remain available from the explicit compiler CLI.

The server now advertises checked root declaration outlines and hover at declaration
names using [compiler-owned symbols](generic-symbols-v2.md). [Direct call navigation](generic-call-navigation-v2.md)
now supports Go to Definition and declared-signature hover. Type-use references outside function headers
remain unavailable. [Generic document formatting](generic-formatting.md) now returns
syntax-preserving edits through the compiler; IR/JS tasks remain refused. Basic keyword/type suggestions remain available and omit
unsupported module syntax. These are static suggestions, not semantic completion.
No checked evidence is guessed from text or borrowed from a different pipeline.

The contributed `sovra` task type is declared for existing source commands. Both
experimental settings remain restricted to trusted user configuration. Compiler
calls use argument arrays and bounded private temporary source files; no shell
interpolation, host grants or source execution is added to diagnostics.

Validation includes real compiler/LSP tests for mutually exclusive initialization,
UTF-16 question ranges, unsaved replacement, standalone scope under a broken
manifest, unsupported capabilities, static completions and legacy isolation. A
historical official VS Code 1.106.0 host on Linux verified diagnostics, Check/Run argument
selection and success, save behavior, and IR/JS task refusal. Other-host execution
and packaged VSIX installation remain separate qualification gates. The new generic
formatting path passes real compiler/LSP tests; updated Electron acceptance has
not yet been run.

Broader generic navigation, package editor integration, JavaScript integration,
remaining E2/G13 requirements and Task Manager readiness are not closed by this
slice. Task Manager itself is unchanged.

See [checked generic local navigation](generic-local-navigation-v2.md) for parameter/local/payload definitions and checked type hover.

See [checked function-header type navigation](generic-type-navigation-v2.md) for record/enum/alias annotation definitions and hover; other type contexts remain open.
