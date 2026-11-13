# Source overlay check transport, version 1

Implemented compiler/CLI foundation for G09. The LSP now uses this for package workspace diagnostics. Checked cross-file function navigation is available as optional metadata; a
persistent workspace cache remains pending.

```text
svr check --format json --overlays-stdin <package-directory>
```

The command reads one framed request through stdin EOF, checks an executable
package graph using existing-entry buffers, and emits one UTF-8 JSON report plus
newline. It never executes package code or writes source files. Disk manifests
and normal resolver/source/AST/interface/evidence/IR admission remain authoritative.
Directories without dependency declarations still use executable package checking
in this mode, rather than the separate application manifest scanner.

The flag cannot combine with `--symbols`, `--service-calls` or human output. The
package argument is at most 4,096 UTF-8 bytes and cannot identify an existing file.
A missing/invalid package root is a compiler diagnostic for a well-formed request.
CLI usage errors use exit 2/stderr without reading a request. Compiler/request
failures use exit 1/JSON; success uses exit 0/JSON. Ordinary check modes are unchanged.

## Request framing

All integers are unsigned 32-bit **big-endian**; lengths count UTF-8 bytes.

| Order | Field |
| --- | --- |
| 1 | Eight magic bytes `SVROVL1` followed by LF (`53 56 52 4f 56 4c 31 0a`). |
| 2 | Caller request ID. |
| 3 | Buffer count. |
| 4, repeated per buffer | Path length, then source length. |
| 5, repeated per buffer | Exact path bytes, then exact source bytes. |

There are no separators between payloads, no escape sequences and no trailing
bytes. Both path and source must be valid UTF-8. Paths must be absolute on the
compiler's host and cannot be empty or contain NUL. Canonical graph membership
and duplicate aliases are checked after complete transport admission. Missing/new
files, manifests and unrelated graph entries cannot be overlaid.

Version-one limits are pinned independently of future library defaults: 32 buffers, 4,096 path bytes each, 512 KiB source bytes each and 4 MiB
aggregate source bytes. Maximum encoded request size is 4,325,648 bytes, including
headers/path/source payload. Reading stops at that maximum plus one byte; an
oversized request fails rather than reading indefinitely. The decoder checks
count/lengths before per-buffer allocations and never sizes storage from an
unchecked declared length. It does retain a bounded encoded input buffer plus
owned decoded strings; these quotas are not a total process-memory bound.

An empty buffer list is valid and checks disk sources. Empty source payloads are
valid transport data and still undergo ordinary parsing/checking. Clients must
close stdin after their single request and apply a process deadline/cancellation.
There is no compiler-side stdin timeout or multi-request daemon protocol.

For a Node client, encode lengths from buffers, not JavaScript string length:

```js
const word = n => { const b = Buffer.alloc(4); b.writeUInt32BE(n); return b; };
const pieces = [Buffer.from('SVROVL1\n'), word(requestId), word(buffers.length)];
for (const { path, source } of buffers) {
    const p = Buffer.from(path, 'utf8'), s = Buffer.from(source, 'utf8');
    pieces.push(word(p.length), word(s.length), p, s);
}
child.stdin.end(Buffer.concat(pieces));
```

The client must enforce valid Unicode and local limits before encoding. The
compiler independently validates every request. Paths and IDs are correlation
inputs, not authorization, content locks or proof that filesystem state is fresh.

## Report and failures

The report retains [check JSON version 1](check-json.md), kind `project`, plus:

```json
"overlay_request": { "schema_version": 1, "request_id": 73 }
```

On malformed framing, invalid UTF-8, unknown magic/version, invalid transport path
or transport-limit exhaustion, E4119 has null location and request ID is null.
Even an apparently intact prefix ID is not echoed from a rejected request.
Valid requests echo their exact ID for both success and graph/source failures.
E4117/E4118 retain the compiler overlay API contract. Other compiler diagnostics
preserve canonical source identity and byte ranges into the supplied unsaved text.
No success, checked navigation or partial IR is published after failure.

Published reports are at most 1 MiB. If the encoded diagnostic report would exceed
the allowance, replace it with one E4120/null-location error and preserve the valid
request ID. Do not truncate JSON or pretend omitted failures constitute success.
Input and output quotas are independent; intermediate report allocation is not
claimed to fit the published-byte allowance. The request does not return full source text, graph caches or IR. Successful
checks may include bounded `package_symbols`, defined by the
[checked navigation contract](package-navigation.md).

The request ID alone does not prevent stale results. A future LSP client must bind
it to the entire source-buffer generation set, discard results after any relevant
edit/close/cancellation and handle missing/unavailable evidence. Non-overlaid
sources/manifests are read from disk during compilation; this is not an atomic
filesystem snapshot. Workspace discovery, diagnostics ownership and checked function definitions are
now implemented. Package editor task targeting and package-specific host acceptance
remain G09 gates.

## Package context preflight

`svr check --format json --package-context <package-directory>` returns check JSON
plus `scope:"package_graph"`, `source_check:false` and nullable `package_context`.
A successful context has `schema_version:1`, canonical UTF-8 `root` and `entries`.
This resolves manifests/dependencies only: context success does not validate or
execute source. Canonical paths must fit 4,096 bytes; output is capped at 1 MiB,
with E4120 on overflow. Invalid roots/graphs preserve resolver diagnostics. The
flag requires JSON, cannot combine with symbols/services/overlays and reads no
stdin. It is a preflight membership view, not a content lock; overlay checking
resolves the graph again and rejects stale/unrelated buffers. LSP results must
still be tied to the current complete buffer-generation set.

## Package task qualification update

G09 existing-entry package tasks and real VS Code workspace acceptance are now
Linux-qualified. See [editor package tasks](editor-package-tasks.md) for save
policy, cancellation and platform limits. `check --executable-package` performs
full package entry compilation without execution; ordinary directory checks keep
the partial application scanner. This flag cannot combine with overlay, symbols
or package-context service modes.

Package context also includes `manifests`, the bounded absolute `sovra.toml`
locators under graph roots. Clients canonicalize these locators for dirty-buffer
identity; they are not a source-validation result or a filesystem snapshot.
