# Sovra IDE integration — experimental

This directory contains a dependency-free Node language server and an installable
VS Code extension. Compiler checking remains in the Rust-owned `svr` executable.
Neither component introduces syntax or changes compiler diagnostic contracts.

## VS Code

Requires VS Code 1.90+, Node.js **22+** and a built/installed Sovra compiler.
From the repository root, build the compiler with `cargo build --locked`.
Install the locally packaged VSIX via **Extensions → … → Install from VSIX**,
or run `code --install-extension /path/to/sovra-language-0.3.0.vsix`.

Set these user settings to executable paths, or put both commands on PATH:

```json
{
    "sovra.compilerPath": "/absolute/path/to/Sovra/target/debug/svr",
    "sovra.nodePath": "/absolute/path/to/node"
}
```

On Windows use escaped backslashes or forward slashes and `svr.exe`.
Run **Sovra: Restart Language Server** after changing settings. Open a `.svr`
file or select the Sovra language for an untitled buffer. Problems and squiggles
update as you edit, including unsaved changes. Initialization/compiler failures
appear in the Sovra Language Server output channel or as operational diagnostics;
they are not reported as successful checks.

Implemented:

- TextMate syntax highlighting, comment toggling, bracket/quote matching.
- Live single-source compiler diagnostics with original codes and UTF-16 ranges.
- Function hover, same-file go-to-definition and function outline, available
  after the whole source checks successfully. Covers root/module functions and
  actual call sites. Standard-library calls show registry signatures with no
  invented file definition. Comments/strings are not treated as calls.
- Parameter/local hover and same-file definition navigation, with branch/loop
  scopes, shadowing and initializer-before-declaration lookup. Function and local
  namespaces remain separate. Explicit annotations retain source spelling;
  inferred locals show compiler-checked scalar, nominal or `Array<T>` types.
  Empty/nested inference holes and oversized labels say `inferred type unavailable`
  rather than guessing a concrete type. Variables
  used as field receivers/array targets navigate to their bindings; field names
  themselves are not local-variable references. The outline remains functions only.
- Basic keyword/primitive-type suggestions (not semantic autocomplete).
- Explicit **Sovra: Check Current File**, **Build Current File as IR**,
  **Build Current File as JavaScript**, and **Run Current File** commands.
  Commands save the active disk file and use VS Code process tasks, without a
  shell. Build output appears in the task terminal; it is not an executable save.
- Workspace Trust gating: untrusted workspaces get syntax highlighting only.
  Compiler and Node executable settings are restricted configuration.

Run executes source with your account's permissions. VS Code Run tasks have
the CLI's normal behavior; terminate them with VS Code's task controls. The
language server itself **only checks** and never runs source.

## Other LSP editors

Server command (standard LSP 3.17 JSON-RPC over **stdio**, no TCP/HTTP):

```sh
node /absolute/path/to/Sovra/toolchain/ide/language-server/server.mjs /absolute/path/to/svr
```

The compiler argument defaults to `SOVRA_COMPILER`, then `svr` on PATH. Any
compiler path containing a separator resolves relative to the server's startup
directory; absolute paths are recommended. Standard output is protocol only.
Supported methods: initialize/initialized, shutdown/exit, didOpen/didChange/
didClose, completion, hover, definition and documentSymbol. Full-document changes (`TextDocumentSyncKind.Full = 1`)
and UTF-16 positions are advertised. Diagnostics use publishDiagnostics with
document versions. Close clears diagnostics. Unknown requests return MethodNotFound.

Neovim 0.11+ example (register `.svr` before starting the LSP):

```lua
vim.filetype.add({ extension = { svr = 'sovra' } })
vim.lsp.config('sovra', {
  cmd = { 'node', '/absolute/Sovra/toolchain/ide/language-server/server.mjs', '/absolute/svr' },
  filetypes = { 'sovra' },
})
vim.lsp.enable('sovra')
```

Emacs 29+ Eglot example (use a suitable installed major mode, or a basic mode):

```elisp
(define-derived-mode sovra-mode prog-mode "Sovra")
(add-to-list 'auto-mode-alist '("\\.svr\\'" . sovra-mode))
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
    '(sovra-mode . ("node" "/absolute/Sovra/toolchain/ide/language-server/server.mjs" "/absolute/svr"))))
(add-hook 'sovra-mode-hook #'eglot-ensure)
```

IntelliJ and Visual Studio need an LSP-capable plugin/client configured with
this stdio command and the `sovra` language ID. Dedicated JetBrains/Visual Studio
plugins are not included. These connection examples are configuration guidance;
those editor hosts have not been tested here.

## Boundaries

Experimental, independent single-source checks only. Open document URIs identify
diagnostics; the server never reads those paths. It checks a private temporary
snapshot with fixed compiler arguments, closed stdin, a ten-second direct-child
deadline and 1 MiB combined output cap. Ordinary shutdown removes temporary files;
forced process/OS termination can leave snapshots behind. This is not a runtime
sandbox or a total memory/CPU/process-tree bound.

Limits: 2 MiB/frame, 4 KiB/header, 32 open documents, 512 KiB UTF-8/document and
4 MiB retained source total. Edits debounce for 150 ms and checks serialize.
New versions cancel previous checks; obsolete results are discarded. Rejected
documents publish an operational diagnostic rather than a clean result. The
client must send a newer valid version (or close/reopen) to retry. Full-sync
messages with a range are unsupported. Invalid framing terminates the server;
known-length invalid JSON/UTF-8 produces a recoverable JSON-RPC ParseError.

Symbol/type evidence comes from `svr check --format json --symbols`, projecting
the compiler's immutable checked types and resolved declaration/use identities.
The IDE does not repeat scope resolution or inference. Rebuild/update the compiler when
upgrading to this extension version. The adapter retains evidence for the checked
document version only; edits, failed checks and close remove it. Incomplete
metadata can still supply the included functions/references; unavailable symbols
return null/empty results. Collection exhaustion leaves valid source diagnostics
clean while explicitly marking metadata unavailable. The adapter validates table
counts, byte/UTF-16 boundaries, strings, definition indices and inferred labels
before exposing them; malformed metadata produces an operational diagnostic.
Definition points at declaration name tokens. Normal check JSON is unchanged.

Symbol queries arriving during a check wait for that version's result, avoiding
cached empty outlines. At most 64 queries wait for up to 11 seconds; cancellation
returns RequestCancelled and document changes return ContentModified.
Closing a document also returns ContentModified to pending queries and clears
diagnostics; reopening the URI may restart its version sequence. Cancelling one
query leaves sibling queries and the shared document check running. Compiler
byte positions are converted together in a forward pass for symbol metadata.

Semantic completion, checked cross-file/type/field navigation, references, rename,
formatting through LSP and debugger integration remain planned. Our browser editor
has not yet been connected to this server. No new `svr lsp` CLI command is added.

## Package workspace diagnostics (experimental)

Trusted VS Code workspaces now pass up to 16 workspace folders to the server.
For existing file documents within those folders, discovery walks at most 64
ancestor directories for `sovra.toml`. The compiler's JSON `--package-context`
query supplies canonical entry membership; the server does not parse manifests.
The query resolves only graph metadata and explicitly reports `source_check:false`.

Existing entry buffers belonging to that graph are checked together through
`--overlays-stdin`, with the normal 32-document/512-KiB-each/4-MiB-total limits.
Unsaved dependency changes affect consumer diagnostics without source writes or
program execution. Rejected related buffers block package checks rather than
silently using old disk content. Multiple open URIs for one canonical entry are
ambiguous and fail closed. Documents outside supported package entries retain
standalone checks; manifests/new files/multi-file modules are not overlaid.

Every accepted open/edit/close in workspace mode conservatively invalidates all
open document generations, cancels active compiler work/pending symbol queries,
clears prior evidence and schedules fresh checks. This is bounded workspace-wide
invalidation, not a persistent incremental dependency cache. Source/manifest watch
notifications and saves also invalidate; VS Code forwards workspace file events
and restarts the server after workspace-folder changes. Disk changes outside
watched workspace folders need another check-triggering event or server restart.

Compiler byte ranges are converted against the correct open source buffer and
published on that document's URI/version. A diagnostic in a closed dependency is
summarized at the requesting document's zero range as an operational package
failure, never paired with consumer text offsets. Results from replaced buffer
objects are discarded. This does not lock manifests or non-overlaid disk files.

Imported function hover/definition and source-owned outlines now use bounded
checked cross-file provenance. Canonical targets and verified source fingerprints
select declarations, never source spelling or foreign import spans. Closed targets
are read through bounded readonly regular-file handles and matched to the compiler
snapshot; unavailable/changed files expose no links. See the
[package navigation contract](../../docs/reference/package-navigation.md).
Successful single-entry checks can still project optional standalone local facts
without overriding package diagnostics. Keyword suggestions remain basic. Editor tasks now resolve the compiler-owned package root and save related source
buffers before launch; dirty manifests block launch. Package Check uses full entry
compilation even without dependencies. **VS Code’s own `task.saveBeforeRun` policy
can also save unrelated editors; choose `never` to retain those drafts.** Sovra
does not change that preference. See [package task behavior](../../docs/reference/editor-package-tasks.md).
The browser editor is not connected to this server. G09 is qualified for its
bounded existing-entry scope on Linux; other platform/release gates remain.

## Develop, package, verify

The extension has **no runtime npm dependencies**. Launch it as a development
extension with `code --extensionDevelopmentPath=/absolute/Sovra/toolchain/ide`.
Build a VSIX using Microsoft's packaging tool (separate development dependency):

```sh
cd toolchain/ide
npx @vscode/vsce@3.6.2 package --no-dependencies --no-rewrite-relative-links --out ../../target/ide/sovra-language-0.3.0.vsix
```

Create `target/ide` first. Keep relative-link rewriting disabled so the source
identity verifier can compare the README byte-for-byte. This packages locally; it does not publish a Marketplace
release or establish ownership of a Marketplace publisher. `sovra-local` is a
local development identity.

From the repo root: `node scripts/test-lsp.mjs` and `node scripts/test-ide-symbols.mjs`. These use real compiler
subprocesses and exercises protocol framing/lifecycle, unsaved changes, Unicode/
CRLF ranges, stale results and admission limits. Extension-host tests are in
`scripts/fixtures/vscode-extension-tests.cjs`, driven by
`scripts/test-vscode-extension.mjs` with the optional `@vscode/test-electron`
development tool. Linux host execution supports a display/Xvfb, or
`SOVRA_VSCODE_HEADLESS=1` for the tested headless Ozone backend. Set
`SOVRA_TEST_UNTRUSTED=1` to test restricted mode separately: the standard test
helper disables Workspace Trust, so the restricted-mode runner deliberately
launches the official downloaded executable directly instead.

Verified on Linux with actual VS Code 1.106.0: activation, unsaved diagnostics,
Unicode ranges, function hover/navigation/outline, parameter/local hover and
shadowing navigation, basic suggestions, edit refresh, restart, and real check/IR/JS/
run tasks with spaces in source paths. A separate restricted workspace verifies
that compiler diagnostics remain disabled. JSON results are written under
`target/ide/`. Windows/macOS VS Code hosts and other editor clients remain
unverified locally. These are local tests, not Marketplace publication.
