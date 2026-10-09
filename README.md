# Sovra

**Version 1.0.0 is a development target, not yet ready for publication.**
Version one must support a production-quality Task Manager CLI built in Sovra.
The current dogfood application runs in both engines, persists tasks, and has no
fixed task-count ceiling; its text-file size and release qualification still need
review. Later versions will add broader
application-platform and third-party publishing capability, each validated by
its own application. The compiler-only and developer-preview release plans are
superseded; see the [version-one gates](docs/V1_DELIVERY_GATES.md) and
[production-readiness plan](docs/PRODUCTION_READINESS.md).

Repository-owned [dogfood applications](dogfood/README.md) exercise the local
compiler and both execution engines. The task manager runs batch and interactive
workflows; [dogfood status](docs/DOGFOOD_STATUS.md) records its persistence and
capacity blockers and the remaining application roadmap.

For the current module/stdlib/package boundaries, see the
[library ecosystem assessment](docs/LIBRARY_ECOSYSTEM_ASSESSMENT.md) and
[real HTTP dependency plan](docs/REAL_HTTP_APPLICATION_PLAN.md). The executable
[library foundations example](examples/library-foundations/main.svr) uses inline
modules. The [local package example](examples/local-packages) now checks, runs,
and builds a consumer using a separate sibling library with private helpers.
Local source resolution is implemented; locks, registries and publishing remain planned.

Sovra is a modern application language for turning ideas into running
software without stitching together a pile of unrelated frameworks.

It is designed to feel simple and readable like Python, safe and performant
like Rust, practical and fast like Go, flexible like JavaScript and TypeScript,
comfortable with declarative and functional programming, and especially clear
to AI agents that need to understand, generate, inspect, test, and modify code.

The north-star developer experience is:

```text
natural-language idea
  -> Sovra code
  -> project structure
  -> svr check
  -> svr test
  -> svr run
  -> application
```

The flagship example is [Fielddesk](examples/fielddesk), a full application
shape that shows models and data, APIs, business logic, UI pages,
authentication, concurrency, and external services in one coherent Sovra
project.

```sovra
model Job {
    id: Id<Job>
    customer: Customer
    status: JobStatus = .open
    urgency: Int
}

route POST "/api/jobs" -> create_job

fn create_job(request: JobRequest) -> Result<Job, Problem> {
    let job = Job.insert({
        customer: Customer.find_or_create(request.email),
        status: .open,
        urgency: score_urgency(request),
    })

    task dispatch(job)
    return Ok(job)
}
```

This repository also contains the complete M0-M11 compiler foundation: a
stable command-line entry point, compiler pipeline boundaries, a lexer, parser,
semantic validation, an explicit intermediate representation, an interpreter
with user-function calls, typed locals, module-aware name resolution, a
standard-library `std` namespace, and a portable JavaScript backend.

The compiler, runtime, and core tooling are implemented in Rust. As Sovra
matures, parts of the standard library and ecosystem can move into Sovra
itself, while the trusted language implementation remains small, inspectable,
and production-oriented.

## Quick start

Development requires Rust; backend execution tests also require Node.js
(CI uses Node 22). The implementation remains pre-stable. See
[full development status](docs/FULL_DEVELOPMENT_STATUS.md) for verified features,
known limits and the current sequence of work.

```text
cargo run -- --version
cargo run -- --help
cargo test
```

The canonical executable is `svr`. `svr run` executes a `.svr` source path.
`svr build` emits human-readable IR by default, and
`svr build --emit js` emits portable JavaScript. `svr --help` lists available
and planned commands; planned commands report a non-zero “not implemented”
message when selected.

`svr check` validates source files or project wiring. For automation,
`svr check --format json <path>` emits a versioned report; see
[automated checks](docs/guides/automated-checks.md) for scope and exit codes.

The first source example is [`examples/hello-world/main.svr`](examples/hello-world/main.svr).
Run it with `cargo run -- run examples/hello-world/main.svr`.
Additional executable examples cover [functions and local inference](examples/functions/main.svr),
[inline modules](examples/modules/main.svr) and [numeric widening](examples/numbers/main.svr).
Every function parameter requires an explicit type; local `let` bindings can infer theirs.
Their lessons are in
[the growing course](docs/course/README.md).

Read [docs/developer-experience.md](docs/developer-experience.md) for the full
idea-to-application walkthrough.

## Repository layout

* `src/` — CLI and compiler-stage modules
* `examples/hello-world/` — first Sovra source example
* `examples/fielddesk/` — full-stack product-direction example
* `docs/` — specification, architecture, roadmap, and contributor guidance
* `.github/workflows/` — CI for formatting, linting, and tests

Read [docs/spec.md](docs/spec.md) for the current language contract,
[ARCHITECTURE.md](ARCHITECTURE.md) for the system boundaries, and
[CONTRIBUTING.md](CONTRIBUTING.md) for development conventions. Agents
continuing the production hardening work should start with
[docs/production-upgrade-flow.md](docs/production-upgrade-flow.md).

## License

Sovra is dual-licensed under either the MIT license or the Apache License,
Version 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
