# Sovra Editor — experimental local preview

A multi-document editor using the real Rust-owned `svr` compiler. No language
syntax, diagnostic schema, desktop UI protocol or Task Manager behavior changes.
This is a developer tool with its own browser editor surface; it does not
pretend the shared leaf/group UI protocol provides a source-code widget.

From the repository root, with Rust and Node.js 22 or newer installed:

```sh
cargo build --locked
node toolchain/editor/server.mjs
```

Open the printed loopback URL. To use another compiler, pass its executable path:
`node toolchain/editor/server.mjs /path/to/svr` (on Windows use `svr.exe`).
Stop the host with Ctrl+C. No dependencies are needed to run the editor.

Implemented:

- Source editing, line numbers, Tab indentation and cursor position.
- New files and a selectable file list with separate drafts and per-file cursor
  selections during the session. Open UTF-8 `.svr` files; download selected source.
- Export/import a version-one `sovra-workspace.json` containing workspace identity,
  active filename and all source drafts. Imports replace the current workspace;
  export it first to keep another workspace. Invalid imports leave drafts intact.
- Up to 64 independent files and 512 KiB combined UTF-8 source. Names use letters,
  digits, `_` and `-` followed by `.svr`, are at most 160 characters and must be
  unique ignoring case. Folder paths and manifests are not accepted.
- Browser-local draft retention across reloads of the same editor origin.
  An ephemeral port means drafts are not reliably recovered after restarting
  the host. Export/import preserves workspace identity across host restarts.
  Existing single-file browser drafts migrate on first load. There is no
  write-back to opened files. Workspace JSON import is capped at 4 MiB.
- Check through `svr check --format json`; click a diagnostic to select its
  UTF-8 byte range correctly in the browser's UTF-16 text buffer.
- Build IR or JavaScript for inspection; Run through the Rust interpreter.
- Check/Build/Run target only the selected source. Diagnostics display its draft
  filename; switching files discards pending results. These independent drafts
  are not a linked package: other files are not passed to the compiler.
- Stop, ten-second subprocess deadline, 1 MiB combined output cap, one active
  operation and rejection of results made stale by source edits.
- Keyboard Check shortcut: Ctrl+Enter / Command+Enter. Responsive layout and
  accessible native editing/buttons/status feedback.

## Execution boundary

**Run trusted code only.** Source runs as your OS account with real file effects.
This tool is not a runtime sandbox. Each operation uses a private temporary
directory, closed stdin, a fixed `main.svr` path, fixed compiler arguments and
no shell. Temporary source is removed after the operation, including failure.
Time/output limits apply to the direct compiler process, not a general process
tree or OS memory budget. No live output streaming or interactive stdin yet.

The server binds only `127.0.0.1` on an ephemeral port and checks the Host header,
exact Origin and a random per-session token for mutations. It serves only four
fixed assets, rejects arbitrary paths/commands, caps JSON requests at 1 MiB and
rejects invalid UTF-8 and unpaired UTF-16 surrogates. Source requiring many JSON
escapes can reach this request limit below the file-picker limit. CSP blocks
external scripts, framing, embedding output and form submissions. Compiler
output and diagnostics enter the DOM as text. This is a local preview: do not
expose it via forwarding or treat its token as protection from local OS users.

## Validation and next steps

```sh
node scripts/test-editor.mjs
node scripts/test-editor-workspace.mjs
# Optional browser regression (Playwright installed separately):
SOVRA_BROWSER_EXECUTABLE=/usr/bin/chromium node scripts/test-editor-browser.mjs
```

The core suite uses actual compiler check/run/build commands and verifies
authentication, malformed requests, output/time limits, Stop and busy recovery.
The browser suite covers Unicode diagnostic selection, literal output, file
open/download, workspace roundtrips/rejection, separate drafts, draft reload,
file switching during execution, Run/Stop and narrow layouts. Screenshots are written
to `target/editor/`. Browser executable configuration is optional if Playwright's
managed Chromium is installed. Linux Chromium is verified; Windows/macOS browser
execution has not been verified locally.

Planned: package-root workspaces, syntax highlighting, search/replace,
language-server completion/hover, automatic draft recovery across host restarts, streamed output,
native desktop packaging and debugger integration. These are not implemented by
this preview. `startEditor` in `server.mjs` is the reusable local host entry point
used by both integration suites; there is no new public `svr editor` CLI command.
