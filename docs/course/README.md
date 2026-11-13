# Sovra course

Updated 2026-10-21. Learn the implemented language with the installed `svr`
toolchain, from a console project to tested libraries and persistent applications.

## Start and learn the language

1. [Install Sovra and create a project](getting-started.md)
2. [Functions, parameter types and local inference](functions.md)
3. [Numbers and widening](numbers.md)
4. [Strings and concatenation](strings.md)
5. [Control flow and Boolean logic](control-flow.md)
6. [Arrays and numeric assignments](arrays.md)
7. [Records and named types](records.md)
8. [Inline modules and function validation](modules.md)
9. [Test ordinary Sovra functions](testing.md)
10. [Format standalone source](formatter.md)
11. [Check toolchain health](doctor.md)

## Build reusable applications

- [Compose library-style inline modules](library-foundations.md)
- [Consume a local library package](local-packages.md)
- [Process arguments, interactive input and exit status](process-input.md)
- [Read and replace UTF-8 text files](text-files.md)
- [Decode text into checked values](text-decoding.md)
- [Display terminal-safe text](terminal-output.md)
- [Build a standalone CLI application](native-builds.md)
- [Practice with the Task Manager and library tests](application-practice.md)

Examples under `examples/` are available in the toolchain package or repository.
The full dogfood suite and acceptance scripts require a repository checkout;
the installed package includes the Task Manager CLI under `samples/task-manager/`.
Older lessons use `cargo run -- ...` for compiler contributors. With an installed
toolchain, replace that prefix with `svr`: `cargo run -- run program.svr` becomes
`svr run program.svr`. Use the working directory stated by each lesson.
Node is needed to execute generated JavaScript, but not to check, run or test
Sovra through the Rust interpreter. Commands using `examples/...` or `dogfood/...`
assume the repository root unless stated otherwise; project-relative commands
run in the directory specified by the lesson. With an installed package, example
paths start at its installation root; the full dogfood tree is not bundled.
See the [examples catalog](../../examples/README.md) for executable versus
checker-only/proposed examples and [current maturity](../LANGUAGE_MATURITY_AUDIT.md)
for implemented scope. Native Windows qualification is separate from source checks.

## Application checker lessons

These teach separately checked, partial application syntax, not application execution:

- [Service and ordinary argument types](service-argument-types.md)
- [Application aliases](application-aliases.md)
- [Application record contracts](application-record-contracts.md)
- [Application arrays](application-arrays.md)
- [Application task returns](application-task-returns.md)

Browser HTTP/UI adapters use Node. Private Windows desktop hosts include a
compiled C#/WPF adapter and a retained PowerShell/.NET compatibility adapter.
An experimental stdio language server and VS Code extension now support checked
function/parameter/local navigation; see [IDE integration](../../toolchain/ide/README.md).
Public cross-platform UI contracts, a registry and WebAssembly remain planned.
Formatter imports/application syntax and
non-Windows writes remain deferred. Native Windows deployment qualification
remains pending. Scanner success is not proof of runnable code.
