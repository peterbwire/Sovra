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
`svr run program.svr`. Run example commands from the directory containing them.
Node is needed to execute generated JavaScript, but not to check, run or test
Sovra through the Rust interpreter.

## Application checker lessons

These teach separately checked, partial application syntax, not application execution:

- [Service and ordinary argument types](service-argument-types.md)
- [Application aliases](application-aliases.md)
- [Application record contracts](application-record-contracts.md)
- [Application arrays](application-arrays.md)
- [Application task returns](application-task-returns.md)

Browser HTTP/UI adapters use Node; a private Windows desktop renderer uses
PowerShell/.NET. Public cross-platform UI contracts, a registry, language server
and WebAssembly remain planned. Formatter imports/application syntax and
non-Windows writes remain deferred. Native Windows deployment qualification
remains pending. Scanner success is not proof of runnable code.
