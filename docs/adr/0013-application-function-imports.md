# ADR 0013: Cross-file application function imports

Status: Option A accepted (2026-10-04). The user replied "okay next" to the
explicit Option A approval request. Implemented for primitive application function
checking and inspection; application execution remains unfinished.

## Boundary

ADR 0006 defines imported service visibility, not ordinary function exports.
ADR 0009 requires module-scoped resolution but does not specify ordinary imported
call spelling. Executable package imports (ADR 0010) remain a separate implemented
boundary. Selecting application callable visibility is a language decision.

## Accepted option A: explicit exported, module-qualified functions

For `use app.helpers`, resolve `app/helpers.svr` through existing project import
validation. Only top-level `export fn` declarations become callable from that
direct importer. Call them as `app::helpers::format(value)`. Bare names continue
to resolve in their declaring file; unexported helpers remain private. No bare
imported names, transitive imports, implicit re-exports or package discovery.

```svr
// app/helpers.svr
fn prefix() -> String { return "value: "; }
export fn format(value: Int) -> String {
    return prefix() + std::to_string(value);
}
```

```svr
// main.svr
use app.helpers;
fn main() { print(app::helpers::format(12)); }
```

Qualified module paths occupy a separate namespace from value bindings, as with
existing executable module-qualified calls. Lexical bindings still shadow bare
names. Canonical source identity plus declaration name identifies a callable;
duplicate imports are idempotent. Cycles use a collected interface graph rather
than recursive body loading. Only fully parsed interfaces may be consumed; errors
retain the owning source. Unsupported application type interfaces remain errors.

Keep existing service visibility and executable package privacy unchanged. This
milestone supplies checking and inspection, not application execution or registry
publication. Future package identities must extend rather than flatten ownership.

## Alternative B

Expose exported functions as bare names through direct imports, diagnosing
ambiguity. This is shorter but adds imported value-name collisions and a separate
precedence decision. It is not recommended for this milestone.

## Acceptance evidence

Positive/negative tests for exports/private helpers, qualified calls, forward and
cyclic imports, duplicate imports, nontransitive visibility, same-name functions
in different files, lexical/module namespace separation, owner-correct diagnostics,
argument/return validation and JSON records. Existing same-file and service
behavior must continue to pass. Do not implement either option before approval.
