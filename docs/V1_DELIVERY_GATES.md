# Version-one delivery gates

Scope explicitly confirmed by the user on 2026-10-03: the full application
platform and third-party library publishing are required before 1.0.0.
Status: **not ready for publication**. Cargo's version is target metadata.
These gates retain M0-M15; they do not narrow M12 or substitute rejection of
all application syntax for implementation.

| Gate | Current state | Required evidence |
| --- | --- | --- |
| Compiler correctness | Partial | Module-local types and iterative alias resolution are implemented. Full retained expression typing, backend parity for compound values and numeric boundaries, and malformed-input/IR coverage remain. ADR 0012 addresses a confirmed array-copy discrepancy. |
| M12 structured checking | Partial | Every supported declaration/body parsed, scoped and typed; service implementations, routes, models, policies and pages have positive and negative cross-file tests. No scanner-only success presented as full validation. |
| M13 native tests | Planned | Package-aware discovery/execution, reliable nonzero failure exits, locations and versioned machine-readable results. |
| M14 application execution | Partial: process arguments, line input and interactive output implemented | Real HTTP requests invoke Sovra handlers; service calls, persistence/auth/page/task integrations execute; errors, limits, cancellation and shutdown are tested. Fielddesk or its verified successor runs end to end. The in-memory task manager does not satisfy this gate. |
| M15 inspection | Partial | Canonical typed symbols, dependency boundaries, application metadata and test results are inspectable with accurate source identity and explicit coverage. |
| Third-party distribution | Local dependencies implemented; publication planned | Immutable versioned archives, integrity checks, lockfiles, offline/cache behavior, ownership/authentication, publication and installation. An independently authored library completes publish/download/consume without compiler changes. |
| Release qualification | Local Windows tests; hosted evidence pending | Exact-commit Linux/macOS/Windows and Rust 1.74 results, extracted artifacts, clean-machine installation/upgrade, compatibility/security review, accurate release notes. |

## Execution order

1. Close confirmed existing-language correctness defects. Obtain approval for
   compound value semantics (ADR 0012); preserve nominal package identity.
2. Build retained typed application structures on the implemented module-local
   type scopes under accepted ADR 0009. Make full-check success meaningful.
3. Specify the missing error/resource and test-declaration contracts, then build
   them through all compiler stages and both supported execution hosts.
4. Deliver locked package artifacts and authenticated publication/installation.
   Registry/version/ownership policies need a concrete ADR before implementation.
5. Implement the host I/O and application lifecycle contracts, then protocol,
   persistence/auth/UI/task libraries and end-to-end application acceptance.
6. Complete inspection and qualify exact release artifacts on supported hosts.

Fundamental semantics and compatibility decisions follow AGENTS.md; routine
implementation and verification proceed under existing approval. No registry
endpoint, credentials, memory model for resources, scheduler or deployment target
is silently selected by this plan. Publication itself is separate from preparing
reviewable code and artifacts.
