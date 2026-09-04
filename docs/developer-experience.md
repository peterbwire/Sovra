# Sovra Developer Experience

Sovra is designed for building full applications in one coherent language.
It should feel simple to read, safe to change, fast to run, and structured
enough that AI agents can inspect a project without guessing where behavior
lives.

The north-star flow is:

```text
natural-language idea
  -> Sovra code
  -> project structure
  -> svr check
  -> svr test
  -> svr run
  -> application
```

## Natural-language idea

Build a field-service dashboard for small operations teams:

* users sign in
* customers request jobs
* technicians accept jobs
* the app estimates urgency and travel time
* invoices are drafted automatically
* managers see live pages without assembling a separate backend, frontend,
  worker queue, auth package, ORM, validation framework, and test harness

In Sovra, that idea should turn into a readable project where data, policies,
routes, pages, jobs, tests, and external services have first-class homes.

## What Sovra code should feel like

Sovra should be:

* simple and readable like Python
* safe and performant like Rust
* practical and fast like Go
* flexible like JavaScript and TypeScript
* comfortable with declarative and functional programming
* designed for AI agents to understand, generate, inspect, test, and modify

That does not mean copying those languages. Sovra should have one voice:
plain declarations, explicit effects, typed data, direct control flow,
structured concurrency, and inspectable application boundaries.

## Project structure

The flagship example lives at [`examples/fielddesk`](../examples/fielddesk):

```text
fielddesk/
  idea.md
  sovra.toml
  app/
    main.svr
    models.svr
    auth.svr
    services.svr
    jobs.svr
    pages.svr
  tests/
    dispatch_test.svr
```

## Command flow

```text
svr check examples/fielddesk
svr test examples/fielddesk
svr run examples/fielddesk
```

`svr check` should validate types, capabilities, routes, model migrations,
auth policies, async boundaries, and external-service contracts before the
program runs.

`svr test` should run unit, integration, page, policy, and service-contract
tests with the same language model the app uses in production.

`svr run` should start the application described by the project manifest.

## Application shape

The example demonstrates the intended integrated surface:

* models and data are declared with `model`
* authentication is declared with `auth`
* API routes are declared with `route`
* business logic is ordinary typed Sovra functions
* pages are declared with `page` and reusable `view` blocks
* concurrency uses `task`, `await`, and `parallel`
* external services are declared once and injected by capability
* tests live beside the app and use the same language constructs

The current Rust compiler in this repository implements the M0-M11 foundation:
lexing, parsing, semantic checks, IR, interpreter execution, and a portable
JavaScript backend for the initial language subset. The Fielddesk example is
the product-direction specimen for the next compiler milestones.
