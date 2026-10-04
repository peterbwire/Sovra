# Application scalar aliases

Run `svr check --service-calls examples/application-aliases` to validate this
checker-only application. It does not start an application runtime.

`Amount` refers forward to `Scalar`, which resolves to `Float`. The same alias
works in service contracts, ordinary parameters/returns and local annotations.
The imported `price` function retains its declaring file's resolved signature.
Alias names themselves are file-local and are not imported or exported.

Use `--format=json` to inspect canonical `Float`, `Int` and `String` evidence in
call records and local bindings. Ordinary project checking remains partial.
