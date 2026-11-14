# Experimental v2 editor mode

The local VS Code extension supports an explicit opt-in setting:

```json
{ "sovra.experimentalV2": true }
```

Run **Sovra: Restart Language Server** after changing it. The default is the
legacy compiler. Compiler and Node executable settings still apply, and workspace
trust is required to start the server, format, or launch tasks. Other LSP clients
can initialize with `initializationOptions: { experimentalV2: true }`; this option
must be a boolean and applies for the server session.

V2 checks the current standalone buffer, including unsaved enum/match syntax,
and publishes compiler diagnostics with UTF-16 coordinates. Completion is a basic
keyword list, not semantic completion. Hover, definition and document symbols now use
[checked function and scope evidence](program-symbols-v2.md), including parameter,
local and match-payload definitions and checked type hover. Checked enum/variant outline entries and variant constructor/match references are
also available. Standard calls, alias/record/field declarations and annotation or
qualified type-prefix uses still have no navigation facts. Legacy package
navigation and overlays are not used in this mode.

Document formatting uses a private temporary copy and `svr fmt --experimental-v2`.
It returns edits without writing the original file or executing the program.
The compiler's token/comment/AST/fixed-point gates apply, including formatting
syntactically valid programs with semantic errors. Changed, closed, cancelled or
rejected document snapshots cannot publish edits. The server bounds concurrent
document actions to four, compiler output to 1 MiB, compiler time to ten seconds,
and transport frames to 2 MiB. Temporary copies are removed after completion;
shutdown waits for active formatting cleanup. VS Code also discards edits after
local version changes or cancellation. Formatter layout follows the compiler;
LSP indentation options do not select another style.

Check, Build IR and Run tasks save and target only the active source file, even
inside a package directory. They pass `--experimental-v2`; no host grants are
added. Package manifests, dependencies, module overlays and lock verification
are outside this editor mode. Build JS explicitly fails through the v2 command;
it does not silently select the legacy backend. [Source-backed native emission](experimental-v2-native.md) is available through
the CLI; the extension does not yet expose a native build command.

Legacy mode retains its package-aware checking, tasks and navigation. Both modes
now expose document formatting through their respective existing CLI formatter.
This slice does not qualify Windows/macOS editor execution, installed VSIX hosts,
package-aware v2 editing, semantic completion, or broader v2 type/field/package navigation.

Protocol regression: `node scripts/test-lsp-v2.mjs`.
Official VS Code host regression: `SOVRA_TEST_V2=1 node scripts/test-vscode-extension.mjs`
(with the optional host dependencies described in the IDE README).

V2 also supports [type-only inline modules](type-modules-v2.md). Qualified enum
and alias constructor/pattern navigation retains module ownership. `mod` is a
basic suggestion for this limited scope, not support for functions inside modules.

Module function drafts can now be formatted through source-owned syntax while
checked diagnostics and navigation still require semantic success. The
[qualified module-function subset](module-functions-v2.md) now supplies canonical
member definitions; invalid or private calls retain errors and clear facts.
