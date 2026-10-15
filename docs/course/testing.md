# Test ordinary Sovra functions

`svr test` checks the entire source or local package, then runs root-level
functions named `test_...`. Each test takes no parameters and returns Unit;
default Unit and aliases resolving to Unit are supported. `main` is not required
or automatically run. Helpers keep ordinary function/module semantics.

```text
svr new hello
svr test hello
svr test --format json hello
svr test dogfood/sovra-budget/tests
svr test dogfood/sovra-notes/tests
```

The console template includes `test_greeting`. The dogfood paths above require a repository checkout; they are not included in the installed sample directory. Reusable dogfood library tests
use actual local dependency packages, nominal records and general functions.

A test succeeds when it returns normally with exit status zero. Write assertions
with ordinary branches and existing status/output APIs:

```sovra
fn add(left: Int, right: Int) -> Int { return left + right; }

fn test_addition() {
    if add(2, 3) != 5 {
        print("expected five");
        std::set_exit_code(1);
    }
}
```

Runtime errors or nonzero status fail the test; the runner continues with the
next one. Status follows the existing last-setter rule. Tests run sequentially
in source order, with fresh frames, local variables, captured output and status.
Discovery excludes inline modules and dependency packages; root wrappers may
call their ordinary helpers. Every body/dependency is checked before any test
executes. `E5001` rejects invalid signatures; `E5002` rejects testless input.

Tests get no program arguments. Stdin reads fail clearly. Existing bounded
file APIs remain available relative to the caller's working directory. There
is no file rollback, filesystem sandbox, per-test timeout or process isolation
in this slice. Use owned temporary/private paths for file effects. First-slice
execution uses the Rust interpreter; native/JS test targets remain future work.

Human output shows safe PASS/FAIL lines, captured output and totals. JSON stdout
contains one [schema-one report](../reference/test-json.md). Exit status is 0
for a nonempty passing suite, 1 for compilation/configuration/test failures,
and 2 for invalid command options. Rust/compiler tests remain a separate suite.
