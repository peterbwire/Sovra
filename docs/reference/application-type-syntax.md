# Partial application syntax

G11 first slice, under accepted [ADR 0009](../adr/0009-m12-checker-contract.md).
This is a Rust library API; it adds no language syntax or CLI checking mode.

`compiler::project::application::syntax::parse(source)` returns shared
`ApplicationTypeSyntax` containing the executable parser's `TypeDeclaration` and
`StructDeclaration` nodes. Aliases and records retain their respective declaration
order, export flags, field annotations and original UTF-8 byte spans. Canonical
module/type identities remain in the existing resolver; these nodes contain
source spellings, not resolved nominal identities. Callers supply file identity.

`source_bytes` is the original UTF-8 source length. `unparsed_ranges` explicitly
lists token runs outside the recognized top-level alias/record declarations,
including imports, functions, services, tasks, application declarations and
nested declarations. Comments/whitespace are trivia rather than parsed syntax.
Ranges are sorted, nonoverlapping token envelopes against the original source.
An empty gap list establishes only type-declaration syntax coverage. It never
establishes successful application checking or semantic validity.

Malformed recognized types, lexer failures and structural-depth exhaustion reject
the request. Unresolved names and semantic errors can still yield valid syntax
nodes. Existing executable alias/record grammar is reused without introducing
model, enum, UI, policy or binding syntax. No filesystem/import resolution, writes
or execution occur through this API.

Optional retention admits 1 MiB source, 4096 combined aliases/records and 8192
record fields. Existing lexer/parser limits also apply. Exceeding retention limits
returns an explicit error from this API. Ordinary application type resolution
continues independently and retains no optional syntax for an oversized file;
metadata absence never converts a failed check to success or skips checking.
Internal module copies share retained syntax with `Arc`.

Current scanner/default project checks remain partial. Structured routes/models/auth/pages, checker coverage integration and eventual
default strict checking remain G11/M12 work. Experimental service-call checking retains
its existing behavior. Task Manager features remain on hold.

## Callable bodies

`syntax::parse_callables(source)` retains ordered `CallableSyntax` nodes with
function/task/service-operation context, original declaration/header/body ranges
and actual executable `Function`/statement/expression AST nodes. Existing
application inspection admits source syntax; each body-bearing declaration then
uses the executable parser. Task keyword tokens adapt to `fn` for parsing only,
with original spans retained. This does not make a task executable or change task
return contracts. Service operation context retains source owner spelling, not a
canonical resolved service identity.

The result includes the same type syntax and aggregate `unparsed_ranges`.
Application imports, service container tokens and bodyless prototypes stay gaps
in this API. Unknown declarations, malformed signatures/bodies and syntax outside
the shared executable body subset reject the whole request. No partial callable
table is returned on failure. Unresolved names/type errors may still have valid
ASTs; even empty coverage gaps do not establish successful checking.

## Service containers and prototypes

`syntax::parse_services(source)` adds `ServiceSyntax` containers and ordered
operations with original ranges. Each operation has a signature and optional body.
Bodyless signatures expose parameter nodes and return annotation spelling, never
an invented executable body. A temporary empty-body token adapter lets the shared
executable parser validate prototype headers; those adapter nodes are discarded.
Body-bearing operations retain the existing callable AST. Empty services and
prototypes with/without a semicolon retain the existing inspection grammar.
Covered type/callable ranges are skipped when indexing containers, so an ordinary
function or alias named `service` cannot be mistaken for a declaration keyword.

The aggregate gaps now exclude recognized service containers, but still include
application imports. Contract resolution, canonical identity, signature type
validity, binding authorization and runtime semantics remain separate. Unsupported
signature forms fail the request, rather than silently becoming covered.

Both APIs admit at most 65,536 non-EOF tokens and 4096 body-bearing callables,
in addition to the type/source limits above. Service retention additionally
admits 1024 containers and 4096 cumulative operations. Exact and exceeded limits
are tested. No ordinary checking path calls these new APIs or falls back to them;
metadata failure cannot bypass existing checking. No filesystem/import resolution,
writes, execution or CLI behavior is added. Current execution evidence is Linux.

## Application imports and aggregate coverage

`syntax::parse_file(source)` retains project-relative dotted `ImportSyntax` nodes,
including source segment order, duplicates, original byte spans and optional
semicolons. It reuses the existing single-line application import grammar;
executable `use dependency::module;` is a different grammar and remains invalid
here. Paths are source spellings, never resolved filesystem targets. Import-like
text inside strings/comments/bodies is not selected. At most 4096 import nodes
are admitted, in addition to existing source/token/declaration limits.

The aggregate `ApplicationSyntax` includes type/callable/service nodes, source
byte/token counts and remaining `unparsed_ranges`. Unknown/malformed application
forms reject the request; an empty gap list means only supported syntax coverage,
not valid names/types/imports/service contracts or M12 completion.

## Opt-in checker integration

`application::check_service_calls_with_syntax(project)` returns
`ServiceCheckWithSyntax`. Its `check` is the existing partial service checker
result. Its separate `syntax` projection contains owning file paths and either
retained `ApplicationSyntax` or an explicit per-file failure. Both use the same
source strings already loaded for application inspection; coverage does not
reread files. This does not lock the filesystem or unify separate scanner reads.

The optional projection admits 128 files, 16 MiB cumulative source bytes and
262,144 cumulative retained non-EOF tokens, with per-file quotas unchanged.
File/byte/token quota failure makes the whole projection unavailable; read,
unsupported-syntax and per-file quota failures stay explicit per-file errors.
Semantic diagnostics are returned independently and are never removed or changed
by successful syntax coverage or metadata exhaustion. Limits apply to optional
retention, not a new blanket budget on existing application checking reads.

Default service/project checking APIs and CLI JSON schemas remain unchanged.
Route/model/auth/page nodes, full coverage-driven validation and M12 acceptance
remain open. No application execution or Task Manager features are added.

## Entry containers and existing binding declarations

`syntax::parse_entry_blocks(source)` adds an independent entry-only projection.
It recognizes top-level `app Name { ... }` headers beginning a source line and
retains original container/member byte spans. Other declarations remain gaps;
this API does not replace `parse_file` or expand the existing checker projection.

Recognized members reuse existing scanner helpers and grammar: API routes, page
routes, scheduled-task bindings, `services`/`data` lists and `auth` targets. Names,
list order/duplicates, decoded paths, schedule text and dotted target spellings
are retained without target resolution, HTTP validation, parameter injection,
policy authorization, generated model operations or runtime behavior. Line-oriented
members keep existing semicolon rules. This adds no route/binding syntax.

Unknown members (including the showcase `title` field) and nested unknown blocks
remain explicit per-entry and aggregate unparsed token ranges. Known declarations
inside unknown nested syntax are never promoted to valid entry members. Malformed
recognized members, lexical/depth failures and unclosed recognized containers
reject the entire request without partial tables. Empty aggregate gaps establish
only entry syntax coverage, never valid contracts or a runnable application.

The API admits 1 MiB source, 65,536 non-EOF tokens, 128 entry containers and 4096
cumulative member/unknown-line records. Source/exceeded token limits and exact/
exceeded container/member bounds are tested, along with the real Fielddesk entry.
Default scanning, checker diagnostics and other syntax APIs remain unchanged.
Next: combine entry and declaration coverage without discarding unsupported forms.
Models/auth/page bodies and full M12 validation remain open.

## Combined entry/declaration coverage

`syntax::parse_combined(source)` retains `CombinedApplicationSyntax` containing
both entry-only and declaration/import syntax against the original source.
Recognized entry envelopes are excluded only from the internal declaration-token
stream. Source text is never masked, sliced into relocated files or rewritten;
all retained AST spans keep their original UTF-8 offsets and scalar line/column
positions, including Unicode and a function following a closing brace on one line.
EOF is always retained even when an entry ends at the source boundary.

The internal range filter is private and receives recognized entry
envelopes (and recognized auth envelopes on the additive auth-aware path). Structural preflight still checks the complete original token stream.
Outside-entry unknown/malformed declarations continue to reject the whole request.
Imports, callables and services hidden inside unknown entry syntax are never
promoted into declarations. Existing type/source/token/entry/member quotas apply
before filtering; excluded tokens cannot evade admission limits.

Combined `unparsed_ranges` contains original tokens unparsed by both projections.
Known entry members can contribute coverage, but unknown fields/nested forms
remain explicit gaps. Empty combined gaps establish supported syntax coverage
only, never semantic checking, binding authorization, application execution or
M12 completion. Declaration-only results are identical when no entries exist.

This is an additive library API. Existing `parse_file`, service/project checks,
opt-in `check_service_calls_with_syntax` and CLI schemas stay unchanged. In
particular, that existing checker projection still uses declaration-only syntax;
combined project integration is available through the separate additive API below.

## Opt-in combined project coverage

`application::check_service_calls_with_combined_syntax(project)` returns
`ServiceCheckWithCombinedSyntax`. The `check` field is the unchanged partial
service checker result; `syntax` separately returns owning file paths and combined
entry/declaration syntax, or explicit errors. It reuses the exact source strings
already loaded for inspection without a second source read. Default APIs, the
older declaration-only projection and CLI schemas remain unchanged.

Both project projections share file/source/token quota accounting: 128 files,
16 MiB source and 262,144 cumulative retained non-EOF tokens. Combined accounting
charges all original entry tokens, including excluded and unknown contents;
filtering cannot hide them from project limits. Per-file failures remain explicit;
cumulative exhaustion makes the whole optional projection unavailable. No partial
coverage table escapes cumulative failure, and semantic diagnostics still return.

Even empty combined gaps do not suppress `E4096` incomplete-inspection errors,
resolve bindings or assert valid names/types. Successful syntax retention and
successful semantic checking are separate results. Exact/exceeded file/byte/token
limits and project diagnostic equivalence are tested. Existing checker read budgets,
filesystem snapshot/locking limits and M12 incompleteness remain unchanged.

## Self-contained local function semantics

`application::check_local_functions(source)` returns `LocalFunctionCheck` with
combined syntax, a count of submitted ordinary functions and core semantic
diagnostics. It builds the executable program from retained ordinary function,
alias and record ASTs, then uses the existing `SemanticAnalyzer`. Annotation,
name/type, call, field, widening, condition and return-path rules therefore match
self-contained executable source. Diagnostics retain original source ranges;
callers supply file identity.

This API explicitly rejects sources with application imports, service containers
or task/service callables because their resolved context is outside this slice.
Malformed/unsupported syntax and retention exhaustion also make the request
unavailable. There is no fallback that omits those declarations or treats tasks
as executable functions. Core AST/depth/diagnostic limits and existing combined
syntax limits apply. No source writes, target execution or filesystem reads occur.

`checked_functions` counts submitted bodies, including bodies with diagnostics;
it is not a count of valid functions. Empty diagnostics establish only local
ordinary-function semantics, not valid entry bindings, coverage of unknown fields,
policy authorization or M12 completion. Entry-only input submits zero functions.
Existing checker APIs, opt-in project coverage and CLI behavior remain unchanged,
including incomplete-inspection errors. Imported/service/task semantic contexts,
model/auth/page bodies and default full validation remain open.

## Structured route/page path validation

`application::check_entry_paths(source)` returns `EntryPathCheck` with entry-only
syntax, submitted route/page counts and diagnostics for supported HTTP methods,
absolute/public path shape and duplicate bindings within the supplied source.
It shares scanner helpers and existing codes (`E4030`, `E4031`, `E4032`, `E4034`,
`E4040`, `E4041`, `E4043`). Diagnostics use exact original member spans and appear
in member source order. Callers supply file identity. The scanner's existing
ordering, target checks and diagnostic locations remain unchanged.

Targets, placeholder argument binding, imported visibility, policies, schedules,
other entry members and callable bodies remain outside this API. Missing target
names can therefore have no path diagnostics while ordinary project checking
still rejects them (`E4033`/`E4042`). Empty path diagnostics establish only this
limited shape scope, never a valid application or route authorization.

Existing entry syntax limits apply. Results admit at most 1024 diagnostics and
1 MiB cumulative UTF-8 message bytes, with existing per-message clipping.
Exhaustion rejects the whole request rather than exposing partial diagnostics as
success. No source reads/writes, runtime execution, CLI changes or new grammar
are introduced. Further binding/context validation and full M12 checking remain.

## Unqualified same-file route targets

`application::check_local_route_targets(source)` resolves route target names only
against retained ordinary function declarations in that source. The result keeps
combined syntax, submitted route count, missing-name diagnostics and ordered
`LocalRouteTarget` records with exact target token spans, original declaration
envelopes and indices into the retained callable table. It does not select a name
from a comment, literal, unrelated declaration or source substring search.

Duplicate callable names, application imports, services/tasks and qualified route
targets explicitly make this limited context unavailable. No file-order winner,
implicit global visibility or cross-module fallback is chosen. Missing local names
use existing `E4033` wording at the exact target token. Source-only callers supply
file identity. Existing syntax limits and 1024 diagnostics/1 MiB diagnostic-message
quotas apply; unsupported context or quota exhaustion returns no partial result.

Resolved declaration identity does not establish handler argument compatibility,
placeholder binding, body/type validity, path validity or runtime authorization.
Those checks remain separate; a resolved handler can still fail local semantic
checking or have an unmatched `:id` placeholder. Page/schedule/auth/model targets
are outside this API. Default scanning, checking, CLI behavior and application
execution remain unchanged. Cross-module context and parameter validation remain.

## Explicit local route placeholder parameters

`application::check_route_placeholders(source)` adds the accepted ADR 0009 rule:
each `:name` in a shape-valid route must match an explicitly named parameter in
its resolved same-source ordinary handler. Missing names report `E4144` at the
original path string token, including quotes/escapes. The API also retains missing
target diagnostics and existing route shape errors. Names are not inferred from
comments, literals or implicit request/session objects.

The result keeps local target identities, submitted placeholder count and bounded
source-ordered diagnostics. Missing targets and invalid route shapes skip name
matching; their diagnostics remain errors. Parameter sets are built once per
referenced handler, so many routes do not repeatedly rebuild large signatures.

Extra handler parameters, parameter type compatibility, HTTP decoding and complete
argument binding remain unvalidated. A matching parameter with an unresolved type
can therefore pass this name-only phase. Repeated placeholder names make binding
context unavailable rather than selecting a decoding rule. Duplicate handler
parameters are rejected by existing signature parsing or by the ambiguity guard.
Imports/services/tasks/qualified targets retain existing unavailable-context rules.

Existing syntax limits and combined 1024-diagnostic/1 MiB message quotas apply;
quota exhaustion returns no partial result. Default APIs, CLI, runtime and source
grammar remain unchanged. Full binding/type/context validation and M12 completion
remain open.

## Local scheduled-declaration target names

`application::check_local_schedule_targets(source)` resolves unqualified schedule
targets to retained same-source ordinary-function or task declarations, matching
the existing scanner's callable categories. Results preserve original binding/
identifier/declaration spans, callable indices and declaration kind. Duplicate
schedule-text/target pairs use shared scanner `E4071` rules; missing names use
shared `E4072` wording at the exact target token. Only unique, existing bindings
are returned. Counts include all submitted schedule declarations. Structured
results retain declaration source order; the scanner retains its existing
target-sorted diagnostic order.

Ambiguous callable names, imports, services and qualified targets explicitly make
this context unavailable. Source/entry quotas and 1024-diagnostic/1 MiB message
limits apply, including no partial results on exhaustion. Scanner default rules,
ordering and locations are unchanged by the shared helper extraction.

Schedule text remains opaque. Argument/return/type contracts, task callability,
timezone/cron/interval interpretation, runtime scheduling and lifecycle are not
established or executed. A bound task can still fail E4131 return checking.
This API adds neither Task Manager software features nor a scheduler. Existing
route restrictions and all default checker/CLI/runtime behavior remain unchanged.
Qualified route/import binding spelling and HTTP conversion still require the
focused designs identified in ADR 0009.

## Auth containers and policy headers

`syntax::parse_auth_blocks(source)` is an independent auth-only syntax projection.
It recognizes line-start top-level `auth Name { ... }` containers and policy
headers using the existing scanner `allow` grammar. Role/action/model spelling,
list order/duplicates and original UTF-8 spans are retained. Other source forms
remain gaps. This standalone API stays independent; the additive auth-aware
composition and project report are described below.

Each `AuthPolicyHeaderSyntax` separates the recognized header from
`PolicyConditionSyntax::Absent` or `Unparsed { span }`. `Absent` means only that no
condition was written, never a permission truth value. Every written `where`
clause remains an explicit unparsed range including its keyword and line tokens.
Unknown provider/user fields, nested forms and continuation lines remain gaps too.
A header therefore cannot hide a guard or establish an authorization result.

Malformed recognized policy headers, lexical/depth failures and unclosed
recognized containers reject the entire request. Unrecognized headers remain
unparsed. No role/model/type resolution, provider binding, condition typing or
runtime grant/deny behavior is introduced. Even empty coverage gaps establish
syntax only, not a valid or executable policy. Scanner/default checker behavior
is unchanged; focused policy-context/type designs remain required by ADR 0009.

Limits admit 1 MiB source, 65,536 non-EOF tokens, 128 auth containers, 4096
nontrivia body lines and 8192 cumulative action/model items. A conditional policy
can also retain a separate condition gap for its body line. Exact/exceeded
container/body-line/item caps, source/token refusal, unknown nested forms,
Unicode/CRLF ranges and real Fielddesk guard retention are tested. No Task Manager
features or application execution are added.

## Auth-aware composition and project coverage

`syntax::parse_with_auth(source)` returns `AuthCombinedSyntax` containing the
existing entry/declaration projection, independent auth nodes and aggregate gaps.
Recognized auth/entry envelopes are privately excluded from declaration admission
using original tokens. Source text and Unicode byte/scalar coordinates are never
rewritten. Container ranges must not overlap, and original source/token/structural
limits apply before exclusion. Hidden functions/imports/services in unknown auth
forms never become declarations.

Coverage is the intersection of the projections' unparsed token ranges. Every
where clause, provider/user field and unknown nested form therefore remains a gap.
Header-only coverage never establishes policy typing or permission. Unsupported
model/page/other declarations outside known containers still reject the whole
source; no partial interface is silently exported. Earlier `parse_combined`,
`parse_file`, local semantic/binding APIs and checker modes remain strict and
unchanged.

`application::check_service_calls_with_auth_syntax(project)` adds a separate
`ServiceCheckWithAuthSyntax` report using the same loaded source strings as the
existing checker. Per-file auth-aware metadata/failures stay separate from the
unchanged semantic result, including E4096 even when syntax gaps are empty.
The shared projection limits remain 128 files, 16 MiB source and 262,144 original
non-EOF tokens; opaque condition tokens count before exclusion. Cumulative quota
failure returns no partial projection, while semantic diagnostics still return.
This is not an authorization engine, default full checking or M12 completion.

Recognized containers must also sit at supported declaration boundaries. Exclusion
cannot join a dangling function/service/type header to later tokens and repair
invalid source. Interrupted-header regressions cover both auth and app envelopes.

## Model/enum headers and local policy model references

`syntax::parse_data_declarations(source)` retains `DataDeclarationSyntax` with
model/enum category, source name/name token, full declaration envelope and an
explicit `unparsed_body` range for every nonempty body. Only basic line-start
named braced headers are recognized. Fields, defaults, optional/generic forms,
enum variants/representation and other headers remain unparsed or unavailable;
no model is silently converted to a record or resolved nominal type. Declarations
generate no constructors, query APIs or other capabilities. Nested declarations
inside unknown bodies are never promoted to top-level names.

This independent syntax API admits 1 MiB source, 65,536 non-EOF tokens and 4096
data headers. Lexical/depth failures and unclosed recognized containers reject
the request. Empty body gaps mean only an empty source body, not type validity.
Earlier aggregate parsers still reject unsupported model declarations; no default
checker/CLI behavior is expanded.

`application::check_local_policy_models(source)` links unqualified policy-header
model references to unique explicit same-source model declarations. Returned
indices preserve auth/policy/list order and duplicate references; missing names
share scanner `E4081` messages at original policy-header ranges. Enums and records
are not models. Imports, qualified names and ambiguous data names explicitly
require unavailable context, with no file-order/global-name fallback.

Both auth/data syntax remain present in the result. Conditions, provider/user
fields, model fields and enum variants remain gaps, and matched names do not
validate roles/actions/types, field access, policy guards or runtime permissions.
Existing syntax quotas plus 1024 diagnostics/1 MiB message bytes apply; exhaustion
returns no partial reference result. The scanner reuses the same missing-model
helper without changing its existing broader wiring scope or E4082 duplicate
policy checks. Policy contexts, typed bodies and full M12 validation remain open.

## Page/view headers and local page-route targets

`syntax::parse_render_declarations(source)` retains line-start top-level page/view
names, source/header/envelope spans and explicitly annotated parameters through
the scanner signature grammar. Annotation strings remain unresolved. Explicit
return text is retained only when written; absent text never becomes default Unit
or a fabricated render result. Nonempty UI bodies remain `unparsed_body` gaps,
including nested view/component calls, loops and bindings. No page/view becomes
an executable Function AST, and no generic/optional/UI type semantics are added.

Malformed recognized signatures, missing annotations, duplicate parameters,
missing/unclosed bodies, lexical/depth failures and exhausted quotas reject the
request. Limits admit 1 MiB source, 65,536 tokens, 4096 page/view declarations and
8192 cumulative parameters. Other source forms remain gaps; earlier aggregate
parsers/checkers still reject unsupported render bodies. Actual render result,
component/binding and lifecycle rules require their focused designs.

`application::check_local_page_targets(source)` links unqualified local page-route
names to unique retained Page/View headers. Ordinary functions are not page
symbols. Imports, qualified targets and ambiguous page/view names explicitly
require unavailable context. Existing scanner `E4042` wording is shared at exact
target identifier spans, and returned indices/category/envelopes preserve source
identity. Existing diagnostic count/message-byte quotas return no partial result
on exhaustion.

Only name identity is checked: path shape, placeholder/extra argument binding,
parameter/result types, UI bodies/components and runtime rendering remain separate.
Default scanner ordering/locations, CLI/checking and runtime behavior are unchanged.
Real Fielddesk headers are tested with their generic-looking annotation text and
all render bodies still explicitly unresolved/unparsed. No Task Manager features
are added.

Render header admission also rejects an interruption by the next supported
declaration before a body opens. A missing page body cannot consume a following
function or line-start view as its own. Function-type/render-result grammar is
not selected by this header-only adapter.

## Explicit local page-route placeholder names

`application::check_page_placeholders(source)` requires every placeholder in a
shape-valid page-route path to match an explicitly named parameter of its resolved
local Page/View declaration. Missing names use `E4145` at the original quoted path
literal. The result retains page/view target identity, submitted placeholder count
and source-ordered diagnostics, including existing missing-target E4042 and page
shape/duplicate errors. Unresolved targets or invalid shapes skip name matching
without hiding their failures.

Parameter-name sets are built once per referenced render declaration. Repeated
placeholder names require unavailable binding context rather than an assumed
multiple-value decoding rule; duplicate parameters fail signature parsing or the
ambiguity guard. Imports/qualified/ambiguous target contexts remain unavailable.

Matching names do not establish parameter/result types, HTTP conversion, implicit
user/session/request injection, extra-parameter binding, UI body validity or
runtime rendering. A matching `id: Unknown` or an unbound extra parameter can
therefore have no name diagnostics. Pages/views keep unresolved render results;
no default Unit is assigned. Existing syntax quotas plus combined 1024-diagnostic/
1 MiB message bounds reject exhausted requests without partial results. Default
checking/CLI and other additive APIs stay unchanged; full M12 validation is open.

## Integrated partial header coverage

`syntax::parse_structured(source)` adds `StructuredApplicationSyntax` containing
entry/ordinary declarations, auth headers, model/enum headers and page/view headers.
It privately excludes only recognized nonoverlapping container envelopes from
ordinary declaration admission at supported original-token boundaries. A following
recognized excluded container may form the next boundary without making older
parsers accept that declaration category. Source text and all byte/scalar spans
remain original; interrupted headers and overlaps still reject the request.

Aggregate `unparsed_ranges` is the intersection of all projections' token gaps.
Model fields, enum variants, UI bodies, guards, provider/user fields and unknown
entry content remain gaps, never interpreted as typed or executable syntax.
Hidden declarations in those opaque ranges are not promoted to ordinary symbols.
Unknown/malformed declarations outside known containers still reject the source;
for example, Fielddesk's ordinary action-DSL function remains unsupported even
though its page/view headers can be retained independently.

All existing per-category source/token/declaration/parameter/item limits apply.
The aggregate counts every original token before exclusion, including opaque
model/render bodies. Empty gaps (possible with empty body/header-only examples)
establish only partial header coverage, never type/authorization/render validity.
Older combined/auth-aware/local-validation APIs remain strict and unchanged.

`application::check_service_calls_with_structured_syntax(project)` exposes this
metadata through additive `ServiceCheckWithStructuredSyntax` and owning file
records. The unchanged checker result uses the same loaded source strings and
retains E4096 regardless of header coverage. Shared project limits remain 128 files,
16 MiB source and 262,144 original tokens; cumulative exhaustion yields no partial
projection while semantic diagnostics still return. This does not add default
full checking, typed bodies, policy permission or UI/runtime behavior.


## Primitive model field checking

The user-approved [first field slice](../design/M12_MODEL_FIELDS.md) is exposed by
`application::check_model_primitive_fields(source) -> Result<ModelPrimitiveFieldCheck, String>`.
This is a Rust inspection API, without a new CLI mode or runtime model support.
`data` retains the existing header-only coverage and opaque body gaps; `models`
adds model-owned field nodes with exact name, annotation, field and declaration
spans. Each model links to its data declaration index. `resolved` links model and
field indices to canonical Unit/Bool/Int/Float/String facts. It is empty whenever
semantic diagnostics exist. No model is converted to an executable record.

Fields require `identifier: SimpleAnnotation`, separated by a comma or a physical
newline after the complete annotation; a single trailing comma is allowed. Blank
lines and comments are trivia. Name, colon and annotation may span lines. LF and
CRLF behave alike. Empty models have zero fields and no construction/layout claim.
File-local primitive alias chains reuse core resolution, including alias-cycle and
unknown-target errors. Duplicate fields/declarations use E3008; undeclared simple
annotations use E3017, at the exact original name/type token respectively.

Malformed fields, missing separators, double commas, defaults, optional/generic/
qualified annotations and declared nonprimitive field types make the entire
projection unavailable (`Err`). Imports require later contextual checking.
Aliases to model/enum/record types cannot produce primitive facts. Existing
line-start data-header admission applies; interrupted headers are refused.
Other declarations and enum bodies retain their independent coverage gaps.

Limits are 1 MiB source, 65,536 original non-EOF tokens, depth 128, 4096 data headers
(including enums under the existing header parser), 8192 cumulative model fields,
1024 diagnostics and 1 MiB cumulative message bytes. Each message uses the core
4096-byte bound. Exhaustion returns `Err`, never a truncated table. Existing
alias/record metadata limits also apply while resolving local type declarations.

Default checking, older syntax APIs and project wrappers are unchanged, including
E4096. Policy references remain names only; field checking grants no access and
does not type guards, enum variants, model values, storage or generated APIs.

A short [field inspection lesson](../course/model-fields.md) shows the source
grammar and Rust API without claiming executable model support.


## Optional project model-field checks

`application::check_service_calls_with_model_fields(project)` returns
`ServiceCheckWithModelFields`: the unchanged partial `check`, plus independent
`model_fields: Result<Vec<FileModelFieldCheck>, String>`. Each file owns its path
and `fields: Result<ModelPrimitiveFieldCheck, String>`. Semantic field errors stay
inside that file's diagnostics and clear its primitive facts. Unsupported syntax,
imports/context or source-read failures stay explicit per-file errors. Local names
never resolve across files or imports merely because the project contains them.

The wrapper uses the same loaded source strings as service inspection, without
another source read. This is shared-snapshot consistency, not an atomic filesystem
snapshot. Old project APIs/CLI, coverage gaps and every original diagnostic,
including E4096, remain unchanged. Field errors are separate and must be inspected
by callers; neither an available report nor a valid local field table establishes
complete project, policy, model-value or runtime validity.

Shared optional project bounds are 128 files, 16 MiB loaded source bytes and
262,144 original non-EOF tokens in retained successful file results, including
opaque bodies. Additional cumulative bounds are 8192 fields, 1024 diagnostics and
1 MiB diagnostic-message bytes. Per-source limits still apply. Admission checks
run before retaining each result; cumulative exhaustion returns no partial project
projection while the original checker result still returns. These are metadata
retention limits, not new blanket limits on the existing source-reading pipeline.


## Ordinary function checking beside structured headers

`application::check_structured_local_functions(source)` returns
`StructuredLocalFunctionCheck` with retained `StructuredApplicationSyntax`, a
`checked_functions` submission count and core semantic diagnostics. Supported
ordinary functions, executable aliases and records reuse the same semantic
submission as `check_local_functions`; signatures, calls, local/field types,
coercions and return-path errors retain core codes/messages/original spans.
The older API keeps its stricter combined-syntax admission path unchanged.

This is still a bounded source-only scope. Imports, services and tasks explicitly
require another checker. Executable references to excluded model/enum/page/view/
auth declaration spellings also return context unavailable. The conservative guard
covers alias targets, record field annotations and identifier tokens inside
retained callables, including declaration names and shadowed spellings. It is an
API admission restriction, not a new namespace or shadowing rule. Strings/comments
and identifiers solely inside opaque bodies do not trigger the guard.

Model/enum bodies, render signatures/results/UI bodies, auth guards/context and
entry bindings are outside this function-only validation. Their source metadata
and opaque gaps remain intact, even if those declarations contain bad types or
unsupported operations. Hidden functions in opaque bodies never enter the checked
program. Empty diagnostics establish only the submitted ordinary executable scope;
a header-only file can submit zero functions. Primitive model-field checks remain
independent. Existing structured source/token/declaration/depth bounds and core
AST/semantic diagnostic limits still apply; exhaustion cannot suppress core errors.
Default project/CLI checking and E4096 remain unchanged. No runtime execution,
model constructors, UI rendering or policy permission is introduced.


## Optional project ordinary-function checks

`application::check_service_calls_with_local_functions(project)` returns
`ServiceCheckWithLocalFunctions`: the unchanged partial `check`, plus optional
`local_functions: Result<Vec<FileLocalFunctionCheck>, String>`. Each record owns
its source path and `functions: Result<StructuredLocalFunctionCheck, String>`.
Core diagnostics retain that file's original source spans. Context/read/syntax
failures remain explicit per-file errors. Callers must inspect both the original
checker result and every optional outcome's diagnostics. An available check is
not a declaration that its functions or the complete application are valid.

The wrapper reuses loaded service-inspection source strings without a second
read; this is not an atomic filesystem snapshot. Each file uses the existing
self-contained source checker: functions are not linked across files, imports
are unavailable, and excluded model/enum/render/auth spellings, services and tasks
still need contextual checking. Model fields, UI bodies, entry bindings and policy
guards stay outside the function semantics. Default APIs/CLI and E4096 are intact.

Shared optional project bounds remain 128 files, 16 MiB source bytes and 262,144
original non-EOF tokens in retained successful results, including opaque bodies.
Additional cumulative admission bounds are 4096 submitted ordinary functions,
1024 diagnostic records and 1 MiB diagnostic-message bytes. Source/parser/AST
limits and the core 256-record semantic diagnostic bound still apply; any core
E3019 exhaustion error remains visible, never converted to valid function facts.
Cumulative excess returns no partial optional table, while the original checker
result still returns. These are optional retention limits, not a change to the
existing project source-reading pipeline or a new application language rule.


## Local route checks beside structured headers

`application::check_structured_route_placeholders(source)` returns
`StructuredRoutePlaceholderCheck`. Its `routes` reuses the existing
`RoutePlaceholderCheck` result: original target declaration identities, supported
route method/path/duplicate checks, E4033 missing handlers and E4144 missing
same-name explicit parameters. Source order and exact original target/path spans
are retained. The old APIs keep their strict combined-syntax parsing paths.

The result moves the parsed combined syntax into `routes.targets.syntax` and
retains `auth`, `data`, `render`, `source_bytes`, `token_count` and aggregate
`unparsed_ranges` separately. No duplicate copy of the combined AST is required.
Target indices belong to `routes.targets.syntax.declarations.base.base.callables`.
All model/enum/UI bodies, guards/context fields and unknown entry members remain
opaque gaps. Hidden functions never supply handlers. Empty route diagnostics
validate this limited binding scope only, even if model fields, handler annotations
or handler bodies are invalid. Page bindings remain independently unvalidated.

Imports/services/tasks, qualified or ambiguous local targets and repeated
placeholders retain their unavailable outcomes. A route target spelling matching
an excluded model/enum/page/view/auth header is also unavailable, including a
same-named ordinary function: this conservative guard does not choose new
namespace semantics. Other missing ordinary names use E4033. Extra arguments,
HTTP types/decoding/injection, policy authorization and runtime routing remain
outside scope. No model constructor or render result type is introduced.

Existing structured source/token/depth/declaration bounds and route diagnostic
bounds (1024 records, 1 MiB message bytes) apply to the complete original source.
Exhaustion exposes no partial check. Default checking, CLI and E4096 remain
unchanged; this is an additive source-only API, not full contextual M12 validation.


## Optional project route checks

`application::check_service_calls_with_routes(project)` returns
`ServiceCheckWithRoutes`: the original partial `check` plus independent
`routes: Result<Vec<FileRouteCheck>, String>`. Each file record owns its path and
`routes: Result<StructuredRoutePlaceholderCheck, String>`. Diagnostics retain
original file-local spans; syntax/read/context failures remain per-file errors.
The wrapper consumes the same loaded source strings as service inspection,
without a second source read or an atomic filesystem-snapshot claim.

Each source keeps its local target/path/placeholder scope. There is no implicit
cross-file target fallback or new cross-file duplicate-path analysis. Imports,
qualified/ambiguous/excluded-header targets, services/tasks and repeated
placeholders still need other context. Handler types/bodies, model fields,
page bindings, guards and rendering remain independently unvalidated. Callers
must inspect the original checker and every optional outcome/diagnostic; an
available report never establishes full application validity or authorization.
Default APIs/CLI and E4096 remain unchanged.

Shared optional bounds are 128 files, 16 MiB loaded source and 262,144 original
non-EOF tokens in retained successful file results, including opaque bodies.
Additional project bounds are 4096 submitted routes, 8192 checked placeholders,
1024 retained diagnostic records and 1 MiB retained diagnostic-message bytes.
Both the combined route diagnostics and nested target diagnostics count toward
retention, including duplicate records/messages already exposed by the source
API. Admission runs before retaining each result. Cumulative excess returns no
partial optional report while the original service result still returns.
Per-source limits remain in force. These bounds limit optional retention and do
not change the existing project source-reading pipeline or language semantics.
