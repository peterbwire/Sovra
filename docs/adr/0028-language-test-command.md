# ADR 0028: First Sovra language test command

Status: **Accepted (2026-10-14).** The user approved the first-slice contract.
Implementation and validation are recorded in the development log.

## Context and implemented foundation

Before this milestone, `svr test` was reserved. The compiler already parses/checks ordinary functions,
and the trusted interpreter can execute their lowered bodies. The new Rust
`run_function_with_host_status` tooling API invokes a chosen zero-argument
lowered function without rewriting IR or replacing `main`. It preserves
existing helpers, function calls, host services and run semantics. An internal
`function_probe` example exercises source/package compilation and this API.

The public command needs explicit discovery, failure, effect and reporting
rules. It does not require new language syntax or a new memory model.

## Accepted first-slice contract

```text
svr test [--format human|json] SOURCE_OR_PACKAGE
```

- Parse and semantically check the entire source/local package before tests
  execute. Use existing package linking and privacy/type rules. This is executable
  compilation, never the partial application scanner.
- Discover only root-entry-file, top-level functions whose names start with
  `test_`. No implicit discovery in dependency packages or inline modules.
  Helpers retain ordinary functions and module calls.
- Discovered functions must have no parameters and return the existing Unit
  type (default Unit or annotations/aliases that resolve to Unit). Invalid test
  signatures fail the run before any test executes. This is a test convention,
  not a change to ordinary function typing. `main` is not required and is not
  automatically run; tests may explicitly call it with its existing semantics.
- Run tests sequentially in source order. Every test starts fresh call frames,
  local state and exit status. An ordinary successful return with status 0 passes;
  a runtime error or nonzero `std::set_exit_code` status fails. Continue with the
  next test after failures. Returned values are not interpreted as assertions.
- First slice uses the Rust interpreter. Native/JavaScript test targets and
  subprocess/process-isolation contracts require later work.
- Capture print output per test so JSON stdout remains one report. Arguments
  are empty and stdin is unavailable: attempts to read a line produce a clear
  runtime error. Tests retain existing bounded text-file APIs and execute relative
  to the caller's working directory. No filesystem rollback or sandbox is promised;
  tests must use their own temporary/private paths for destructive operations.
- No new builtin assertion API. Tests can use ordinary comparisons/branches,
  print failure details and select a nonzero exit status. Reusable assertion
  helpers can be ordinary Sovra library functions.
- Exit 0 when at least one test exists and all pass; exit 1 for source/package
  errors, invalid signatures, no discovered tests or any test failure; exit 2
  for malformed invocation/options. A testless input must not look like evidence.
- Human output shows per-test PASS/FAIL and totals, with safe display of captured
  text. JSON uses a new test-report schema 1 with input identity, tests, captured
  output, exit status, runtime error or null, aggregate counts and success.
  Source compilation failures retain existing diagnostic codes and spans.

## Implementation milestones after approval

1. Shared discovery/Unit resolution behind compiler/tooling APIs, with valid and
   invalid signatures, alias/import coverage and no-execution-on-error tests.
2. Capture host and sequential runner preserving errors, status resets and
   file behavior; ordinary helper calls and explicit `main` calls remain correct.
3. CLI invocation, human/JSON output and subprocess regressions. Add a verified
   course lesson and template tests, then installed/out-of-repository acceptance.
4. Test existing reusable dogfood packages through real Sovra execution. Keep
   compiler Rust tests and application behavior independent and passing.

## Approval

The user approved this first command's invocation, discovery/signature convention,
execution/effect rules, status categories and reporting boundary together.
AGENTS.md requires pausing before compatibility decisions. The Rust execution
foundation and public command are implemented. Later test targets/process-isolation
contracts require their own reviewed extensions.
