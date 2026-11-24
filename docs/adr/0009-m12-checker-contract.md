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

## 2026-11-12: G11 retained type syntax checkpoint

The application type collector now retains the executable parser's alias/record
nodes rather than discarding their syntax after resolving types. Shared optional
metadata records original spans and explicit nontrivia unparsed ranges. A bounded
Rust library API exposes this partial syntax without claiming semantic validity
or full application coverage. Unknown declarations stay unparsed; malformed
recognized types fail. Retention exhaustion never disables ordinary inspection.
This implements part of the accepted structured-parser direction without new
language grammar or changing default project-check compatibility. See the
[type syntax contract](../reference/application-type-syntax.md).

Next: structured callable/body coverage and further M12 declaration areas. Route
bindings, policy contexts and other unapproved syntax still need their focused
designs before implementation. This checkpoint does not complete G11 or M12.

## 2026-11-12: callable/body and service syntax checkpoint

Bounded library APIs now retain callable signatures and executable statement/
expression ASTs with original byte spans, using current application admission
and executable parsing. Task token adaptation introduces no runtime semantics.
Service containers retain ordered operations and bodyless signatures without
exposing synthetic bodies. All APIs keep aggregate unparsed token ranges and
reject malformed/unsupported syntax or exhausted quotas without partial results.
Semantic name/type checking and service authorization remain separate even when
gaps are empty. No default project-check compatibility changes occur.

Next: import nodes and coverage integration; route/model/auth/page nodes and
focused unapproved binding/policy designs remain. This does not complete G11/M12.

## 2026-11-12: application import nodes and opt-in checker coverage

`syntax::parse_file` now retains single-line dotted import nodes with source
segments/order/duplicates/spans and aggregate coverage. It reuses existing
application import grammar without resolving filesystem targets. Unsupported
syntax fails rather than becoming covered; empty gaps never establish semantic
validity.

`check_service_calls_with_syntax` returns the existing service-check result plus
separate optional coverage built from the exact same loaded source strings.
A private snapshot-inspection result supports reuse without a second source read.
Default APIs/CLI schemas stay unchanged. Coverage admits 128 files, 16 MiB source
and 262,144 retained tokens, with existing per-file limits; cumulative exhaustion
makes the projection unavailable while semantic diagnostics remain intact.
Regression tests cover grammar/order/spans, exact/exceeded quotas, malformed and
unavailable files, semantic errors despite full syntax coverage and equivalence
to existing checker results. Coverage limits are not new blanket read budgets on
the partial checker. Structured route/model/auth/page coverage and eventual
default strict validation remain G11/M12 work. Task Manager remains unchanged.

## 2026-11-12: structured entry containers and existing binding members

Added `syntax::parse_entry_blocks` as an independent bounded library projection
for source-line top-level app headers and existing route/page/schedule/services/
data/auth members. It reuses current scanner grammar helpers and preserves
source spans, list order/duplicates, decoded paths and target/schedule spellings.
No binding/type/HTTP/policy resolution, parameter injection, new syntax or runtime
behavior is introduced. Unknown fields and nested forms remain explicit gaps;
malformed recognized members reject the whole request. Entry-only coverage never
asserts full validation, and existing `parse_file`/checker/default APIs are unchanged.

Limits admit 1 MiB source, 65,536 tokens, 128 app containers and 4096 cumulative
member/unknown-line records. Regression tests include exact/exceeded limits,
Unicode/CRLF spans, preserved grammar, malformed declarations, nested unknown
forms and the real Fielddesk entry with its title explicitly unparsed. Next:
combined entry/declaration coverage. Model/auth/page bodies, concrete unapproved
binding/policy designs and full M12 acceptance remain open. Task Manager remains
unchanged.

## 2026-11-12: combined entry/declaration coverage

`syntax::parse_combined` now composes entry and declaration/import projections
against original tokens. Only recognized entry envelopes are privately filtered
from declaration admission; no source masking/rewriting changes byte or Unicode
scalar positions. Full-token structural preflight and existing quotas apply
before filtering, and EOF is retained. Source ranges unparsed by both projections
remain gaps, including unknown entry fields/nested forms. Known entry coverage
never promotes hidden imports/callables/services or authorizes runtime behavior.

Regressions cover the real Fielddesk file, explicit unknown title retention,
Unicode/CRLF and inline-after-entry positions, unknown nested content, EOF-only
entries, strict existing APIs and identical ordinary declaration projections when
no entries exist. Default checkers/CLI and existing opt-in checker projection stay
unchanged. Next: project integration of combined coverage. Full M12 checking,
model/auth/page bodies and platform gates remain open; Task Manager is unchanged.

## 2026-11-12: opt-in combined project coverage

Added `check_service_calls_with_combined_syntax` with separate checker and combined
syntax results from the same loaded source strings. Shared generic quota accounting
serves both the unchanged declaration-only projection and the new projection.
Limits remain 128 files, 16 MiB source and 262,144 retained tokens; original entry
tokens, including unknown contents, count before filtering. Per-file failures
remain explicit and cumulative exhaustion returns no partial projection.

Project regressions confirm unchanged diagnostics/files, E4096 preserved despite
empty combined gaps, Unicode unknown-field ranges and semantic errors preserved
on metadata exhaustion. Exact/exceeded file/byte/token limits are exercised,
including large unknown entry contents to prevent hidden-token quota bypass.
No default checker, CLI, syntax grammar, runtime or Task Manager behavior changed.
Further structured M12 validation, model/auth/page bodies and platform gates remain.

## 2026-11-12: core semantic validation of self-contained local functions

Added `check_local_functions(source)` as an additive source-only API. It builds
an executable Program from retained ordinary function/alias/record ASTs and uses
the core SemanticAnalyzer, preserving canonical primitive/type/name/call/field/
condition/return-path rules and original diagnostic ranges. The result separates
combined syntax, submitted function count and local semantic diagnostics.

Application imports, service declarations and task/service callables explicitly
make this limited request unavailable rather than being silently omitted or
misclassified. Entry bindings/unknown fields remain unvalidated even with empty
local diagnostics; entry-only input submits zero functions. Existing AST/depth/
diagnostic and combined syntax limits remain active. No default checker, CLI,
runtime, grammar or Task Manager behavior changed.

Parity regressions compare standalone and entry-adjacent local functions across
valid records/aliases/widening and invalid annotations, calls, fields, conditions,
unknown names and return paths, including exact diagnostic codes/messages and
Unicode/CRLF source ranges. Additional tests cover missing semantic contexts,
invalid syntax and continued strict behavior of existing inspection. Imported/
service/task contexts, model/auth/page bodies and full M12 validation remain open.

## 2026-11-12: structured route/page shape validation

Added source-only `check_entry_paths` over retained app members. Extracted scanner
route/page method/path/duplicate helpers so structured validation reuses existing
rules/codes instead of creating a second dialect. Scanner target resolution,
locations and ordering remain unchanged; the additive API emits exact member
ranges in source order. Counts describe submitted declarations, not valid bindings.

The limited scope deliberately excludes target resolution, placeholder arguments,
import visibility, policies, schedules and body validation. Missing targets still
fail ordinary project checks even when this API has empty path diagnostics.
Existing entry quotas plus 1024 diagnostics/1 MiB cumulative message bytes bound
results. Exhaustion rejects the whole request without partial success.

Regressions compare scanner codes/messages across ten invalid cases, preserve
Unicode/CRLF member spans, verify target-check separation and exercise exact/
exceeded diagnostic counts and cumulative byte exhaustion. No syntax/default
checker/runtime/CLI or Task Manager behavior changed. Further structured binding/
context checks and full M12 acceptance remain open.

## 2026-11-12: unqualified local route target resolution

Added `check_local_route_targets` against retained ordinary function declarations
in the same source. Resolution returns original route/identifier/declaration
ranges and callable indices; it never selects comments/literals or file-order
winners. Missing names retain E4033 scanner wording at exact identifier tokens.
Ambiguous names, imports, services/tasks and qualified route targets explicitly
make the limited context unavailable. Diagnostic count/message-byte exhaustion
returns no partial result under existing quotas.

Tests cover Unicode/CRLF spans, misleading literal/comment spellings, declaration
indices, exact/exceeded diagnostic counts, byte exhaustion and scanner code/message
parity. Separate tests prove resolved name identity does not establish body/type
validity or placeholder/parameter compatibility. Existing source/checker/CLI and
runtime behavior are unchanged. Cross-module/parameter context and model/auth/page
bodies remain M12 work; Task Manager remains unchanged.

## 2026-11-12: explicit local placeholder parameter names

Added `check_route_placeholders` implementing the accepted requirement that each
route placeholder match an explicit handler parameter name. Missing names use
E4144 at the original path string token; missing target and shared route shape
errors remain present. Parameters are indexed once per handler and diagnostics
are source ordered. No implicit request/session injection, HTTP decoding/type
rule, extra-parameter binding or task execution is introduced.

Repeated placeholder names explicitly require unavailable binding context rather
than choosing semantics. Duplicate parameters fail existing signature parsing or
the ambiguity guard. Existing unsupported target contexts and diagnostic quotas
remain active; exhaustion yields no partial result. Tests cover original Unicode/
CRLF ranges, missing/matching names, retained missing-target/shape failures,
unresolved types/extra parameters outside scope, duplicate/repeated ambiguity,
exact/exceeded diagnostic counts and cumulative byte exhaustion. Default checker/CLI,
language grammar and Task Manager behavior stay unchanged. Full binding/type/
context validation and M12 completion remain open.

## 2026-11-12: local scheduled-declaration target checks

Added `check_local_schedule_targets` using retained ordinary/function task
identities and exact identifier spans. Shared scanner helpers preserve duplicate
schedule-text/target E4071 and missing-target E4072 rules/messages without changing
default scanning. Only unique existing bindings return indices; ambiguous/import/
service/qualified contexts remain explicitly unavailable. Existing source/entry
and diagnostic quotas reject exhausted requests without partial bindings.

Regressions cover source identity/kinds, Unicode/CRLF ranges, scanner parity,
unsupported contexts, exact/exceeded diagnostic count and cumulative bytes.
Separate return-contract tests demonstrate that a bound task may still fail
E4131. Schedule text, argument/return/type validity, task invocation, scheduler
execution and lifecycle are outside scope. Task Manager software is unchanged.
Qualified route binding spelling and HTTP conversion still need focused design;
further authorized structured M12 areas remain available without inventing those
semantics. Full M12 acceptance and platform qualification remain open.

## 2026-11-12: auth containers and policy-header syntax

Added independent `syntax::parse_auth_blocks`, reusing current scanner policy
header grammar and retaining role/action/model lists, original spans and explicit
condition coverage. Every where clause stays `Unparsed` and remains in aggregate
gaps. Provider/user fields, nested forms and outside declarations remain gaps;
malformed recognized policies and unclosed containers reject the request. No
role/model/context/type checking or runtime authorization is established, even
when a header matches scanner metadata or all syntax gaps are empty.

Limits are 1 MiB source, 65,536 tokens, 128 containers, 4096 nontrivia body lines
and 8192 cumulative action/model items. Tests exercise exact/exceeded metadata
bounds, source/token refusal, malformed/nested syntax, Unicode/CRLF ranges,
Fielddesk provider/user/guard gaps and scanner header parity without erasing an
explicit false condition. Existing combined/checker APIs and CLI behavior are
unchanged. Policy contexts/typing still need the focused designs required above;
model/page bodies, broader structured integration and full M12 acceptance remain
open. Task Manager and application runtime behavior are unchanged.

## 2026-11-12: additive auth-aware source/project coverage

Added `syntax::parse_with_auth` and `check_service_calls_with_auth_syntax` without
relaxing earlier syntax/checker APIs. Recognized auth and entry envelopes are
privately filtered from declaration admission using original tokens; overlapping
container ranges are rejected. Shared token-gap intersection preserves every
where clause, provider/user field and unknown nested form. No hidden function,
service or import is promoted from unparsed auth content, and no policy validity
or permission result is asserted. Unsupported outside model/page declarations
still reject the source.

The project report uses the same loaded sources and generic file/source/token
quotas as existing projections, while semantic diagnostics remain unchanged,
including E4096 with empty metadata gaps. Tests cover guard/context preservation,
hidden nodes, Unicode/CRLF coordinates, EOF containers, strict older APIs,
project diagnostic equivalence and original opaque guard-token accounting at and
above cumulative limits. Policy typing/context design, model/page coverage and
full M12 checking remain open. Task Manager and runtime behavior are unchanged.

## 2026-11-12: data declaration headers and local policy model references

Added independent `parse_data_declarations` with model/enum category, name-token
and envelope spans, keeping every nonempty field/variant body explicitly unparsed.
No model-to-record translation, generic/optional/default grammar, enum runtime
representation, nominal type assertion or generated model/query API is introduced.
Limits admit 1 MiB source, 65,536 tokens and 4096 data headers; nested unknown
forms are not promoted and unclosed recognized containers reject the request.

`check_local_policy_models` resolves unqualified source-header model references
only to unique explicit same-source Model declarations. Enums/records do not
satisfy them; imports, qualified names and ambiguous data names remain unavailable
contexts. A shared scanner helper preserves E4081 wording while the source API
uses original header spans. Conditions/context fields and data bodies stay gaps,
so name matching never validates or authorizes a policy. Existing diagnostic
count/byte quotas return no partial result on exhaustion.

Tests cover source categories/spans, opaque bodies, exact/exceeded declaration
and diagnostic caps, lexical/source/token refusal, enum/model distinction,
ambiguity/import boundaries, scanner parity and real Fielddesk model/enum headers.
Default checking, aggregate coverage, CLI/runtime and Task Manager behavior are
unchanged. Typed model/page bodies, policy context/type design and full M12
acceptance remain open.

## 2026-11-12: render headers and local page-route names

Added independent `parse_render_declarations` using existing scanner signature
structure for explicit page/view parameters. Names/header/envelope spans and raw
annotations are retained; every nonempty render body remains unparsed. Absent
return text is not converted to Unit, and pages/views are never fabricated as
executable Function nodes. No render result, generic/optional type, UI component,
binding or lifecycle semantics are introduced.

`check_local_page_targets` resolves unqualified source-local page-route names only
to unique Page/View declarations, not ordinary functions. Imports, qualified and
ambiguous contexts remain unavailable. Shared scanner E4042 rules/messages use
exact target spans in the additive API while default behavior remains unchanged.
Source/token/declaration/parameter and diagnostic count/byte quotas bound results
with no partial result on exhaustion.

Tests cover explicit/unknown/missing annotations, no Unit defaults, opaque/nested
UI bodies, exact/exceeded metadata/diagnostic limits, source spans, declaration
kinds, scanner parity and real Fielddesk page/view headers. Placeholder/argument
binding, result/type checking, typed bodies and full M12 integration remain open.
Default checking/CLI/runtime and Task Manager behavior are unchanged.

## 2026-11-12: explicit local page/view placeholder parameters

Added `check_page_placeholders` for the accepted explicit-name requirement on
page-route placeholders. Missing names use E4145 at original path literal spans;
existing missing-target/page shape errors stay visible. Only resolved shape-valid
Page/View targets submit placeholders. Signature name sets are cached per retained
declaration; diagnostics are source ordered and bounded by existing count/byte
quotas. Exhaustion returns no partial binding result.

Unresolved parameter types, extra parameters, HTTP conversion, implicit context
injection, render-result typing and UI bodies remain unvalidated. No default Unit
is assigned. Repeated placeholder semantics remain an unavailable context, while
duplicate parameters fail existing signature parsing or the guard. Tests cover
Page/View kinds, Unicode/CRLF spans, missing/matching parameters, preserved target/
shape errors, old inspection strictness and exact/exceeded diagnostic count/byte
budgets. Default checker/CLI, runtime and Task Manager behavior are unchanged.
Full binding/type/context and M12 validation remain open.

## 2026-11-12: integrated partial source/project header coverage

Added `parse_structured` and `check_service_calls_with_structured_syntax` as
additive APIs composing entry/declaration, auth, data and render header projections.
Recognized container envelopes must be nonoverlapping and begin/end at supported
original-token declaration boundaries; the next excluded container is recognized
by its compiler-owned range, not by globally weakening older parser categories.
No source rewriting changes Unicode coordinates, and interrupted declarations
still fail before ordinary admission.

Every opaque field/variant/UI body, condition/context field and unknown entry
form stays in aggregate gaps. Hidden declarations are not promoted. Unknown
ordinary DSL bodies outside excluded containers still reject the request.
Shared snapshots and file/source/original-token quotas serve the project report
without suppressing semantic errors or E4096. Empty header-only gaps never assert
type, permission or render validity; earlier APIs and default checking stay strict.
Tests cover composition, exact source coordinates, boundary failures, real
Fielddesk guard/data-body gaps, original opaque-token quota accounting, per-file
failures and unchanged project diagnostics. Typed/context/render/policy designs
and full M12 acceptance remain open. Task Manager/runtime behavior is unchanged.

## 2026-11-12: proposed first typed model-field slice

The [model-field proposal](../design/M12_MODEL_FIELDS.md) makes separator grammar,
primitive/file-local alias scope, diagnostics, limits and acceptance fixtures
reviewable. It is proposed, not implemented or approved. Newline-or-comma fields
are recommended to preserve existing proposed model examples; a comma-only
alternative and its migration impact are stated. Runtime representation, defaults,
generic/optional/model-valued fields and policy authorization remain outside scope.
Implementation waits for the required syntax/type decision.


## 2026-11-12: first model-field slice approved and implemented

The user approved option A of the [field design](../design/M12_MODEL_FIELDS.md),
superseding the pending-approval checkpoint above. The additive source checker
now retains separate model-owned fields, newline/comma separators and primitive/
local primitive-alias facts. Core alias errors and exact E3008/E3017 field errors
remain separate from bounded syntax/context refusal. Unsupported/exhausted
requests expose no partial facts; semantic errors clear the primitive table.
Header-only/aggregate/default APIs remain unchanged and retain E4096. Model values,
enum variants, runtime/generated APIs, policy typing and full M12 remain open.


## 2026-11-12: optional project model-field outcomes

The approved source field rules now have an additive project wrapper using the
same loaded source strings as service checking. Each result owns its file path;
field diagnostics and unsupported/read failures stay separate from unchanged
service diagnostics. Imports still require contextual checking. Shared project
file/source/token bounds and cumulative field/diagnostic/message-byte admission
prevent partial retained tables on exhaustion. E4096/default APIs remain intact;
this neither checks complete models/policies nor completes G11/M12.


## 2026-11-12: ordinary semantics beside structured headers

Additive `check_structured_local_functions` now submits ordinary function bodies
and executable aliases/records through the existing core semantic path beside
opaque model/enum/auth/page/view headers. The old local-function API stays strict.
Original spans and remaining gaps are retained. Imports/services/tasks and uses
of excluded declaration spellings explicitly need contextual checking, including
conservative refusal of shadowed spellings. No model/enum namespace or construction
semantics is selected. Body/header metadata never makes model fields, UI bodies,
entry bindings or policy guards valid. Default checking and E4096 remain unchanged;
full G11/M12 contextual checking remains open.


## 2026-11-12: optional project ordinary-function outcomes

`check_service_calls_with_local_functions` now exposes the self-contained source
checker through file-owned outcomes from the same strings as service inspection.
Original service diagnostics/E4096 are unchanged. Project admission bounds cover
files/source/original tokens, submitted functions and diagnostic count/message
bytes, refusing partial optional tables on cumulative exhaustion. Core semantic
exhaustion remains an error. No cross-file linking, imports/contextual application
semantics, model operations, rendering or policy permissions are introduced.
Full M12 validation and host/release qualification remain separate gates.


## 2026-11-12: local route rules beside structured headers

Additive `check_structured_route_placeholders` now uses original structured
headers beside shared same-source ordinary route target/path/placeholder rules.
Private helpers preserve older parser admission and rule behavior. Original
syntax/diagnostic spans, declaration indices and aggregate gaps are retained
without duplicating the combined AST. Excluded-header target collisions explicitly
need context, selecting no namespace/constructor semantics. Hidden handlers,
model fields, enum variants, UI bodies, guards and handler body/types stay outside
this binding scope. Default checking/E4096 are unchanged; full M12 remains open.


## 2026-11-12: optional project route outcomes

`check_service_calls_with_routes` now exposes file-local structured route checks
from the same loaded sources as service inspection. File-owned diagnostics and
unavailable contexts remain independent from unchanged service diagnostics/E4096.
Shared file/source/original-token budgets and cumulative route/placeholder/
diagnostic admission prevent partial retained reports on exhaustion. Diagnostic
budgets include both combined and nested target copies. No implicit cross-file
binding, global path analysis, HTTP typing, model/runtime or policy semantics is
selected. Full contextual M12 and host/release qualification remain open.
