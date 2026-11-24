# Installed Sovra developer tools

The Windows toolchain includes the browser editor, stdio language server and
VS Code extension sources. Normal compiler and packaged-application execution
does not require Node. These optional developer tools require Node.js **22+**.

From a PowerShell window with the installed Sovra bin directory on PATH:

```powershell
svr-editor -Check
svr-editor
```

Use `-NodePath 'C:\path with spaces\node.exe'` when Node is not on PATH. The editor
prints a loopback URL; open it in your browser and press Ctrl+C to stop the host.
Run only trusted source: editor Run retains your real account permissions.

For another LSP editor, use the Node executable directly with arguments
`C:\...\Sovra\developer-tools\launch.mjs`, `lsp`. This preserves binary UTF-8
stdio framing. PowerShell's text pipeline is not an LSP transport, so configure
the editor to spawn Node directly instead of embedding a PowerShell pipeline.
The launcher resolves its own installed `bin/svr.exe`, validates the manifest's
compiler version and accepts only its fixed modes. It never loads project scripts.

If `manifest.json` lists a VSIX, install that file through VS Code's **Install
from VSIX** command. Set `sovra.compilerPath` to this installation's `bin/svr.exe`
and `sovra.nodePath` to Node, then use **Sovra: Restart Language Server**.
The source extension also works with
`code --extensionDevelopmentPath='C:\...\Sovra\developer-tools\ide'`.
An installed compiler is paired with the same-package editor/LSP source; the
explicit version check catches incompatible versions, not arbitrary local edits
or every different build sharing the same version. Verify package checksums.

`editor/README.md` and `ide/README.md` describe experimental features and limits.
Their repository-development examples are separate from these installed paths.
No network download, automatic VSIX installation or Node installation occurs at
tool startup. Missing Node/compiler or a version mismatch fails explicitly.
