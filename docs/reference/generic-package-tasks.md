# Experimental generic package tasks

To make VS Code Check/Run operate on the generic package containing the active
source, enable both user settings and restart the Sovra language server:

```json
{
    "sovra.experimentalGenerics": true,
    "sovra.experimentalGenericPackageTasks": true
}
```

The new setting is restricted to trusted configuration, defaults to false and
requires generic mode. Existing generic mode keeps its standalone task behavior
unless this additional setting is enabled. V2 mode remains separate.

In package-task mode the server asks the compiler for the selected package's
entry, registered modules, dependency sources and manifests. It does not parse
manifests in JavaScript. Discovery stays within configured workspace roots. Outside that discovery scope,
files use the standalone path. Check/Run receives the package root and
`--experimental-generics`; normal compiler package checking/execution then
captures and validates actual source bytes. An active registered module selects
its owning package. A file outside any package remains standalone. Inside a
discovered package, unregistered sources, broken manifests and invalid context responses
refuse the task instead of falling back to standalone execution.

Task preparation saves the active source, then dirty open source buffers selected
by the compiler, including dependency modules. It does not save unrelated files
or manifest drafts. Dirty selected manifests, cancelled saves, duplicate
canonical editors, changed targets and continually changing sources refuse.
Existing bounded retries, document-version checks, cancellation and timeout
controls remain in force. Saves completed before a later refusal can remain.
The compiler resolves again after source saves; the
returned paths are metadata hints, not immutable snapshots or authority to reuse
previously read bytes.

This setting changes task targeting only. Diagnostics remain standalone unless
the separate [generic package diagnostics](generic-package-diagnostics.md) setting
is enabled. [Generic package function navigation](generic-package-navigation.md) is available
with package diagnostics; type/field/local reference coverage remains open. It does not enable data/collection profiles,
host-policy grants, IR/JS tasks, automatic execution or native Build tasks.

## Compiler metadata protocol

```sh
svr check --experimental-generics --format json --package-context ./app
```

The response uses the existing top-level check schema with `kind: "project"`,
`scope: "package_graph"`, `source_check: false` and
`profile: "experimental-generics"`. Its `package_context` has schema version **2**,
canonical `root`, `sources` (entries and registered modules) and `manifests`.
Legacy entry-only context retains schema version **1** and its `entries` field;
the two decoders refuse each other's profiles. No source lexing, type checking,
lock verification, source writes or execution occurs during metadata selection.
Malformed source can therefore have a successful metadata response; it will
still fail normal Check/Run.

Metadata requires JSON and a package directory. Duplicate context flags and
combinations with symbols, locks, collection/data modes or non-check commands
are usage errors. The task transport admits at most 1024 sources and 1024
manifests, 4096 UTF-8 bytes per path, 128 KiB aggregate selected path text and a
bounded JSON response below the existing 1 MiB transport limit. Failure returns
no partial context. The compiler's larger general source-inventory ceiling is
unchanged; the stricter task limit applies to this protocol.

Real Linux compiler/LSP tests verify registered-source membership, related-only
saves, dirty-manifest refusal, actual package Check/Run, malformed metadata,
strict standalone defaults, unregistered-source refusal and broken-graph refusal.
Rust tests accept exactly 1024 sources and reject 1025. Actual VS Code Electron
host acceptance and additional OS runtime support remain separate qualification.
