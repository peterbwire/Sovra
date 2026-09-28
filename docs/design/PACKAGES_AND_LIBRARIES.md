# Third-party libraries and packages

Status: Partial foundation. Accepted ADR 0010 has a graph-only local dependency
resolver (`project::packages::resolve`). Executable dependency linking, publishing
and registry access are not implemented. The user explicitly requires developers to be able to
publish reusable Sovra libraries for other projects to consume.

## Architectural contract

A library must be an ordinary Sovra package, without needing changes to the
compiler or Rust-owned builtin registry. Applications and libraries declare
their dependencies and public API in package metadata and source exports.
The exact manifest and import spellings require a future design decision;
examples of proposed syntax must not be presented as accepted syntax today.

Resolution proceeds through three separate responsibilities:

1. The package resolver selects declared dependencies and records an exact graph.
2. The module resolver selects a module within a resolved package.
3. The symbol resolver applies exports, visibility and lexical binding rules.

A resolved package identity includes its source and exact selected revision or
version, not just its display name. A module identity includes that package
identity and its package-relative module path. Symbols additionally carry their
declaring module identity. Filesystem paths are source locations, not portable
package identities. Same-named modules from unrelated packages must not alias.

Current `ProjectImport` and `ServiceIdentity` records are early local structures,
not the final package identity contract. Extend them deliberately when a real
dependency graph exists; do not encode external identities by concatenating
unchecked filesystem paths or by flattening all symbols into one global map.

## Dependency sources and reproducibility

Plan for local path, Git and registry dependencies. Local dependencies support
library development before a registry exists. Git dependencies must lock an exact
commit; registry dependencies must lock an exact release and content identity.
Version selection, transitive conflict handling, feature selection and whether
multiple versions may coexist need explicit resolver policies before coding.

A lockfile records enough information to reproduce the selected graph. Builds
use the locked graph; dependency updates are explicit. Cached, fully available
dependencies should permit offline builds. Missing dependencies and incompatible
toolchain requirements must produce actionable, machine-readable diagnostics.

## Package boundaries and visibility

The current project-root containment rule becomes a per-package-root rule.
Only a declared dependency resolved by the package resolver authorizes access to
another package. Each package's module loader still rejects paths escaping its
own root, including canonical-path escapes. Declared local dependencies outside
the application directory are therefore possible without permitting arbitrary
cross-directory imports.

Consumers see public exports, not every discovered declaration. Private helpers
remain private. ADR 0006's application-service import model is not a blanket
export policy for all library symbols. Re-exports and dependency aliases need
specified rules; an import must not expose transitive dependencies implicitly.
Preserve the builtin `std` namespace and bare `print` compatibility alias.

## Author and consumer workflows

Authors need library scaffolding, documented exports, tests, API documentation,
version/license/toolchain metadata and local package validation. Publication
should preview the exact included files and validate dependency metadata before
upload. Published artifacts need stable content identity and ownership controls;
registry protocol, authentication, namespaces, yanking and release policy remain
future designs. No public registry endpoint or publishing command is chosen here.

Consumers need dependency add/update/remove workflows, compatibility diagnostics,
lockfile review and reproducible installation. Tooling should expose package,
module and symbol identities so language-server and agent features use the same
graph as the compiler. Packages must work through the trusted compiler stages,
not by installing compiler-specific business primitives.

## Incremental delivery and acceptance

Keep the established M0–M15 milestone numbers. Package work is a dependency track
alongside them, sequenced after reliable cross-file module and visibility rules:

1. Resolve package-qualified modules and exports using local library fixtures.
2. Add manifest dependency records, local path dependencies and a lockfile format.
3. Add exact Git and registry resolution, caching and integrity verification.
4. Add publish validation and registry workflows with reviewed ownership rules.
5. Integrate dependency-aware tests/docs and structured inspection with M13/M15.

Before claiming third-party libraries work, build a consumer using a separate
library without compiler modifications. Verify public/private access, nested
dependencies, same-named modules, version conflicts, relocation, locked/offline
builds, missing artifacts and package-root containment. Publishing additionally
requires a verified publish/download/consume round trip. Scaffolding and metadata
alone do not meet these acceptance criteria.
