# Generic package test suites

Generic Test now supports registered modules and local dependencies on qualified
Linux/procfs hosts:

```sh
svr lock --experimental-generics ./project
svr test --experimental-generics --locked --format json ./project
```

`test_generic_package(captured, label, compilation, policy, suite_limits)` consumes
one immutable graph snapshot. Types, signatures and every original body are
checked once with cumulative frontend admission. Tests are source-order `test_`
functions in the root package entry only: nongeneric, parameterless, Unit-returning.
No main is required or implicitly invoked. Dependency entries and registered
modules supply helpers; their test functions are not discovered automatically.
Invalid unused dependency bodies still refuse the entire suite before execution.

Every selected root passes specialization, concrete checking, literal decoding,
layout and IR preflight before any VM executes. Programs are then recompiled from
the same shared body proof and immutable bytes, retaining one lowered program at
a time. No source/manifest rereads or parser/type-checker repetition occurs during
replay. A replay compilation failure stops remaining execution and retains earlier
reported outcomes; it does not roll back earlier results. Runtime failures are
individual test outcomes, and later tests continue with fresh VM state and policy.
Per-test instruction allowance is fresh, not depleted by earlier tests.

`GenericPackageTestLimits` independently hard caps tests at 32 and concrete
function instances across both passes at 8192. Preflight reserves each root's
instances for both passes; per-root discovery is capped by remaining reservation,
and replay cannot allocate more instances than that root reserved. Existing finite
compilation-stage and per-test VM limits remain active. Limits can be reduced;
zero never selects a fallback. Count/empty-capacity admission uses E5003; a reduced
compiler-stage profile can refuse with E3123 before any VM executes. Invalid test
signatures use E5001, and no root tests uses E5002. These are logical resource
limits, not physical-memory/process-isolation or hard-deadline guarantees.

Human and schema-one JSON reports reuse the existing test protocol. Runtime
outcome errors include the originating captured file, scalar line/column and E3125
code, including dependency-module failures after Unicode/CRLF. Output remains
empty in this host-free profile. Status is 0 only for a nonempty passing suite,
1 for input/compile/discovery/runtime failure and 2 for invalid arguments.

Locked CLI suites capture and verify the graph lock before frontend checking or
VM execution. Missing/stale/malformed/unsafe locks return an empty package test
report without rewriting the lock. Suite execution continues from captured bytes
when disk inputs are subsequently removed or changed. No I/O hosts or source
reread fallback are introduced. Standalone file tests retain their separate source
suite API/profile.

The [generic package example](../../examples/generic-packages/README.md) now has
two root tests that consume exported generic helpers and Option propagation.
Multi-file native Build now uses [versioned package images](generic-package-native-v2.md);
single-entry native/source tests keep their previous contracts. Consolidated
qualification and remaining G13 functional capabilities stay open. E1/E2/G13 remain open;
editor polish is deferred and Task Manager is unchanged.

[Versioned generic package native images](generic-package-native-v2.md) now support source-backed native graph builds, with in-memory startup checking and separate embedded/capture authority.
