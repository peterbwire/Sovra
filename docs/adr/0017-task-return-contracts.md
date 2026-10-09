# ADR 0017: Task return contracts in application checking

Status: Accepted and implemented (user continued with recommended rule on 2026-10-09).

## Context

M12's opt-in application checker parses `task` bodies and resolves explicit
return annotations, but does not validate returned values or fallthrough.
Consequently `task build() -> Int {}` and `task build() -> Int { return "bad"; }`
can pass task return checking. The current specification explicitly leaves
omitted task results undefined. Scheduled tasks are still declarations, not
executable application jobs.

## Proposed rule

- A task without `-> Type` has a `Unit` return contract, matching ordinary
  functions. Falling through and bare `return` are valid; returning a value is
  invalid.
- An explicit `-> Unit` has the same behavior. An explicit non-Unit result
  requires a return on every supported control-flow path. Both arms of an
  `if`/`else` must return; a `while` alone does not prove return completeness.
- Every explicit returned expression must have a resolved type compatible with
  the declared result. Preserve nominal record identity and the existing
  Int-to-Float widening rule. An unknown expression type fails closed.
- Reuse E4131: declaration range for possible fallthrough or unresolved
  annotation; return expression/keyword range for mismatch or unresolved value.
  Keep task parameter diagnostics E4134 and existing ordinary/service return
  behavior unchanged.
- This rule applies to opt-in M12 project checking only. It does not make tasks
  callable, scheduled or executable; those runtime/lifecycle contracts remain
  separate M14 work.

## Consequences and verification

This tightens compatibility: previously accepted task bodies with mismatched
returns will fail `check --service-calls`. Valid and invalid tests must cover
implicit/explicit Unit, scalar and nominal results, fallthrough, branches,
loops, unknown expressions, and source/JSON diagnostic locations. Documentation
must distinguish return validation from scheduler execution.

Alternative: leave omitted task results unspecified and validate only explicit
annotations. That avoids one compatibility change but leaves task bodies with
unannotated return values without a coherent type contract.
