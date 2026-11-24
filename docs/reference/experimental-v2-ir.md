# Experimental linked IR inspection

```sh
svr build --experimental-v2 --emit ir main.svr
svr build --experimental-v2 main.svr
```

Both commands check and lower a regular `.svr` source or supported [single-entry
package](experimental-v2-packages.md), then print a
complete bounded inspection artifact. No `main` is required. All source functions
are lowered and validated, including unused helpers. This mode never executes
code, reads program stdin, grants file services or changes application status.
Programs containing host calls or nonterminating loops can be inspected without
executing them. Compiler source reads still use the existing bounded reader.

The header is `sovra-ir v2 inspection-only`, followed by the materialization-model
version. Output includes flat enum payload-type tables and variants (distinguishing
nullary alternatives from Unit payloads), sorted record/field schemas, function
return types, bindings, uses, conversion contexts, expression operations, roots
and function instructions. Explicit conversion types, canonical function targets,
enum dispatch/payloads, scope lifetimes and branch targets remain visible.
Ranges are original root-source byte offsets; the target filename is omitted.

Identifiers are local inspection labels: `f` functions, `e` enums, `t` payload
types, `d` bindings, `u` uses, `c` conversion contexts, `v` expression values, `r`
roots, `m` match snapshots, `s` scopes and `pc` function instruction positions.
Function IDs follow root source order. Per-function labels restart in each
function. Internal request/environment ownership nonces and source labels are
never emitted. Hash-backed record schemas are sorted explicitly. Repeated or
independent checks of the same source produce identical text for the same
compiler, irrespective of request identity or target filename.

Strings, identifiers and operators are quoted/escaped; printable Unicode remains
UTF-8. Canonical core types and context categories use their compiler diagnostic
spelling. This is an inspection format, not a stable machine schema, executable
image codec or proof of backend compatibility. Native/JavaScript v2 encoding,
decoding and executable compilation remain separate work. Old image codecs and
default `build` output are unchanged.

## Admission and publication

The CLI uses `ProgramSourceLimits::DEFAULT`, `ProgramLoweringLimits::DEFAULT`
and `ProgramIrLimits::DEFAULT`: at most **4,194,304 final UTF-8 output bytes**,
including escaping, metadata and newlines. Every write checks arithmetic and
remaining capacity before appending. No partial successful artifact or unlimited
fallback is returned when compilation or rendering refuses. These logical quotas
do not impose an OS memory limit or hard compilation deadline.

The complete artifact is retained before stdout publication. Source/read/lowering/
render/publication failure returns 1; success returns 0. Output publication may
partially write before an OS error; the command handles write/flush failures
without a panic. Usage errors return 2 with no artifact. Only the v2 flag, optional package `--locked`, one
optional `--emit ir`, and one input select inspection. Native `--emit exe` now
requires `--output PATH`; see [source-backed native builds](experimental-v2-native.md).
JS emission, output paths for IR, host grants, JSON and duplicate options refuse.
Unsupported package dependencies/modules are configuration failures as documented
in the package reference.
Use `--` before a dash-prefixed source filename; it does not introduce program
arguments for build.

`CheckedProgramSource::render_ir(lowering, limits)` exposes the same renderer for
Rust callers. Lowering failures retain their typed `ProgramExecutionError` inside
`ProgramIrError::Lowering`; output exhaustion is `ProgramIrError::Limit`.
Unavailable sealed ownership/table agreement refuses with `ProgramIrError::Invalid`.
No partial text is exposed by this API. This advances E1 compiler inspection;
multi-source packages/imports, IDE integration, v2 backend encoding and G13 completion remain open.

For supported package directories, optional `--locked` verifies the captured
inputs before compilation or execution. See [v2 locks](experimental-v2-locks.md).
