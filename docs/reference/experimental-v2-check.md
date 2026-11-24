# Experimental root-program checking

`svr check --experimental-v2 [--format human|json] [--symbols] <file.svr|package-directory>` checks the opt-in
v2 root-program subset: enum/record/alias declarations, typed functions, linked
function calls, exhaustive matches and structured conditionals/loops, plus
[type-only inline modules](type-modules-v2.md) referenced by root functions. At least
one function is required; a `main` entry is not required for checking.

This command only reads and checks source. Calls to printing, file operations,
input and exit-status functions are checked without execution or host services.
Successful checking does not qualify a backend or guarantee resource admission
when a program is later lowered or executed.

Default `check`, `run`, `build` and their language contracts remain unchanged.
The check mode supports [single-entry packages](experimental-v2-packages.md) on
qualified guarded-reader hosts; dependencies/registered modules, the
application scanner remain unavailable. Standalone JSON checks can explicitly
request [checked root-function symbols](program-symbols-v2.md) with `--symbols`.
The separate [experimental run mode](experimental-v2-run.md) executes source
with finite VM limits and no stdin/file hosts by default. Pass `--` before a source path that starts with a dash. Duplicate flags,
unsupported options and missing arguments are usage errors.

Human output escapes control characters in paths. JSON retains the existing
[check report schema](check-json.md) version 1 and source check kind. Diagnostics
use original root-source byte coordinates and the actual requested path; internal
declaration labels never become filenames. The projection publishes at most 64
bounded diagnostic messages, using E3019 when truncated. E3120 reports unsupported
root syntax, E3121 a missing function and E3122 unavailable checked evidence.
Component quotas use E3025; failures without source coordinates have null locations.
Existing syntax, semantic and bounded source-read diagnostic codes are preserved.

Exit statuses are 0 for successful checking, 1 for source or read failures and 2
for usage errors. Standalone sources use the existing bounded strict-UTF-8 reader and must
be regular `.svr` files. Checking does not provide an execution sandbox.

G13 remains partial. Multi-source package/dependency linking, package-aware IDE
contexts and executable IR/JS backend integration remain open. Source-backed native
emission and standalone IDE selection are available through their explicit modes. See
[checked program execution](program-execution-v2.md) for the explicit library API.

Directory inputs keep check schema 1 kind `project` and use captured entry text
with original entry-file diagnostics; standalone files keep kind `source`.

For supported package directories, optional `--locked` verifies the captured
inputs before compilation or execution. See [v2 locks](experimental-v2-locks.md).
