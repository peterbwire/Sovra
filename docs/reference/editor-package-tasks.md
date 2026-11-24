# Package editor tasks

The experimental VS Code extension resolves Check, Build IR, Build JS and Run
through compiler-owned package context. Existing graph entries target their
package root, including nested entry files. Other files retain standalone tasks.
All commands use ProcessExecution arguments and an explicit working directory.
Package Check uses `check --executable-package`: full entry compilation, including
packages without dependencies, without execution. Ordinary directory Check keeps
its existing partial application-scanner behavior.

The server advertises `experimental.sovraTaskTarget`. Its `sovra/taskTarget`
request takes `textDocument: {uri, version}` for an open document. The versioned
result contains `schema_version: 1`, `package`, `target`, `cwd`, `sources` and
`manifests`. Paths are absolute, bounded and canonicalized for editor identity.
Graph metadata does not imply successful source checking. Stale requests return
ContentModified; cancellation and changes retire requests. At most four physical
operations remain outstanding, with bounded compiler/request timeouts.

The extension saves the active source first, then resolves the graph and saves
only dirty graph source buffers. Dirty related manifests block launch and require
an explicit user save. Other source drafts are outside this helper's save scope.
Preparation admits at most 128 file buffers, 32 dirty graph sources and three
stabilization attempts; failed saves, duplicate canonical editors, changed target
roots and continued edits abort. It never parses TOML to guess a graph.

**VS Code also applies its own `task.saveBeforeRun` setting.** Its default can
save unrelated editors. Choose `never` to retain unrelated drafts while Sovra
saves graph sources. The extension does not change this user preference.

This is not an atomic filesystem transaction. The compiler revalidates the actual
disk graph when the task starts. Workspace trust is required. Background checks
and navigation continue to use bounded unsaved overlays instead of saving files.

Real-host verification uses `SOVRA_TEST_PACKAGE_WORKSPACE=1` with
`scripts/test-vscode-extension.mjs`, including a nested entry, dependency buffers,
manifest refusal, all four tasks and restart. `SOVRA_VSCODE_CACHE` optionally keeps
the official host download between runs. Linux qualification does not establish
Windows/macOS, MSRV or release qualification.
