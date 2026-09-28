# ADR 0009: M12 checker completion contract

Status: Accepted (2026-09-28). The user explicitly approved all six decisions.
Implementation is incremental and incomplete; acceptance does not establish M12 completion.

## Evidence

M12 requires checking manifests, modules, routes, models, auth policies, standard
library calls, service contracts and page bindings before execution. Current
project checking is a partial scanner. Fielddesk additionally uses unresolved
types, query expressions, closures, UI declarations, concurrency and retries.
Its successful wiring check cannot establish application correctness.

ADR 0006 import visibility and ADR 0008 explicit service-body scope remain
accepted. This proposal does not reopen those decisions.

## Decisions requested

### 1. Meaning of successful project checking

Recommended: the default project check must parse every discovered project source
file and validate all declarations and bodies supported by the checker. Unknown
syntax and unresolved references are errors, never silently skipped. A separately
identified partial inspection result may expose useful metadata, but cannot report
full check success. Bring service-call checks into the default check when this
contract is implemented; preserve the existing flag as a compatibility alias.

Alternative: retain default wiring-only success and require an opt-in strict check.
This leaves the ordinary command insufficient for M12's before-execution promise.

### 2. Application annotation rules

Recommended: use the executable subset's canonical primitive names: Unit, Bool,
Int, Float and String. Require annotations on function, task, service operation,
page and view parameters; retain local inference and omitted function/service
return types meaning Unit. Pages and views have separately defined render results,
not an inferred Unit result. Reject unknown annotation names. Do not silently
make Text an alias for String; migrate examples or declare a library type explicitly.

Alternative: maintain a separate application-only annotation/type dialect.
That adds translation rules and permits the scanner/compiler to disagree.

### 3. User-defined type identity and imports

Recommended: record (`type`), model and enum declarations have nominal identities:
two declarations with identical fields are still different types. Resolve names
from the declaring module and direct imports. Report ambiguous imported names;
do not choose one by file order or expose transitive imports. Reserve internal
identity components for package, module and declaration, so future libraries do
not share one global name table. A module-qualified spelling must be specified
before implementing qualified type syntax.

Alternative: structural type equivalence or a project-global type namespace.

### 4. Domain types and model operations

Recommended: business types and capabilities such as Money, Email, GeoPoint,
database queries and payment operations belong to declared libraries. No
capitalized-name exemption, magic external type, or automatically generated
`Model.find`/`Model.insert` API. A model declaration alone must not authorize such
calls. Undefined Fielddesk names must be supplied by declarations/imports or fail.

Alternative: compiler-owned application types and automatically available model APIs.
This conflicts with the project's library-first architectural direction.

### 5. Routes, pages, views and policies

Recommended: route handlers and page/view targets resolve through module scopes;
validate their argument names/counts/types and field references using declared
signatures. Route placeholders must have an explicit matching handler/page
parameter. Do not implicitly inject user/session/request objects: bindings must
be declared. UI components and policy helper capabilities require declared
signatures too. Unknown components, fields and helpers are errors.

Alternative: convention-based injection and permissive dynamic UI/policy names.
Concrete binding syntax, HTTP decoding and policy context signatures still need
focused proposals; approval here chooses explicitness, not those spellings.

### 6. Milestone acceptance and compatibility

Recommended: keep the existing M12 scope. Completion requires structured parsing,
name/type validation and valid/invalid integration fixtures for every listed area,
including routes, models, auth, services and pages. Unsupported application forms
remain errors and documented gaps; do not mark M12 complete by rejecting all
remaining features or renaming the scanner. Update Fielddesk to verified syntax
and declare its dependencies; retain any unimplemented showcase separately and
clearly label it planned. Runtime execution belongs to M14; library publication
remains a required separate track, not a feature claimed by M12.

Alternative: explicitly narrow M12 to wiring validation and defer the rest to a
new milestone. This changes the roadmap and is not recommended.

## Approval boundary and follow-ups

AGENTS.md requires: "Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions." Decisions 1–6 select the checker
contract and architectural policies, not every missing language feature.
Generics/Option/Result, record initialization, closures, query syntax, UI binding
syntax and policy typing require concrete follow-up designs before implementation.
No memory model, asynchronous execution, database behavior, registry protocol or
public release is authorized by accepting this proposal. Routine parser,
diagnostic and test work proceeds under accepted rules.
