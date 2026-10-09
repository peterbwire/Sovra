# Task return contracts in application checking

Status: M12 checker support. Task execution and scheduling remain planned.

Under ADR 0017, an application task without `-> Type` has a Unit result. A
non-Unit task must return a compatible value on every supported path:

```svr
task refresh() { return; }

task next_id(current: Int) -> Int {
    if current > 0 { return current + 1; }
    else { return 1; }
}
```

`svr check --service-calls <project>` reports E4131 when a non-Unit task can
fall through, when a returned value has the wrong or unknown type, or when a
Unit task returns a value. Int may widen to Float; record results retain nominal
identity. A loop by itself cannot prove that a result is always returned.

This check validates declarations and inspected bodies. It does not start a
background worker, call tasks, or prove that a scheduled task runs.
