# Fielddesk

Fielddesk is the flagship Sovra developer-experience example.

It starts from a product idea and shows how Sovra should describe the complete
application in one project: models, APIs, auth, services, business logic,
concurrency, pages, and tests.

This example is intentionally written in the target application syntax. The
current compiler foundation supports the smaller executable subset in
[`examples/hello-world`](../hello-world), while Fielddesk defines the shape
Sovra is growing toward.

## Idea to App

```text
Idea:
  "When a customer requests help, assign the best available technician,
   draft an invoice, and show the job on a live operations dashboard."

Commands:
  svr check examples/fielddesk
  svr test examples/fielddesk
  svr run examples/fielddesk

Application:
  authenticated operations dashboard
  typed customer/job/invoice data
  API routes
  background dispatch work
  live UI pages
  external maps and payment services
```

## Files

```text
app/main.svr       Application entry and wiring
app/models.svr     Data models
app/auth.svr       Authentication and authorization policies
app/services.svr   External service contracts
app/jobs.svr       Business logic and concurrent work
app/pages.svr      UI pages and views
tests/*.svr        Sovra-native tests
```
