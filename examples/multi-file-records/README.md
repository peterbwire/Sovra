# Registered record interface example

**Partial G12: source-owned compiler and executable CLI on supported guarded Linux
hosts.** Run `svr check examples/multi-file-records/app`,
`svr run examples/multi-file-records/app` or `svr build --emit js` with that path.
Rust callers may use `packages::compile_registered_records` directly.
The independent consumer uses the separate shapes library's registered geometry
module, exported nominal Point, primitive field alias and constructor alias.
Interpreter and Node JavaScript regressions verify output `25` then `1.5`.

Fields may resolve to Unit/Bool/Int/Float/String or visible declared records via
source-local aliases, as well as existing public standard result records. Field references retain nominal IDs without layout
expansion; unsupported nonregistered field contexts fail explicitly. Distinct
packages do not unify equal-shaped records; exports/direct imports govern type
spellings. The primitive-only compiler API keeps refusing records. Explicit source
locks and inventory-aware package formatting are implemented; complete context/
overlay/navigation/editor integration remains planned.

On supported Linux/procfs hosts, explicitly generate a source lock with
`svr lock PATH_TO_APP`, then use `svr run --locked PATH_TO_APP` or
`svr check --executable-package --locked PATH_TO_APP`. `lock --check` verifies
without writing. Package formatting includes registered sources and leaves locks
untouched; regenerate explicitly after reviewed changes. See the
[lock contract](../../docs/reference/package-source-locks.md).
