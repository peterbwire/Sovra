# ADR 0008: Explicit service implementation scopes

Status: Accepted (2026-09-28); user approved explicit service calls. This decision advances M12, not service
execution or full application type checking.

## Current blocker

The contract scanner records operation signatures and whether bodies exist.
Structured inspection rejects nonempty service implementation bodies with E4096
because their scope rules have not been specified. Skipping them while reporting
complete inspection previously hid invalid calls and was fixed. M12 needs a
defined scope before safely inspecting those bodies.

## Recommended option A: reuse explicit file and lexical scopes

An operation body sees the same same-file/direct-import service declarations as
an ordinary function in that file, under accepted ADR 0006. Its parameters and
locals shadow service names in lexical order. Nested blocks do not leak bindings.
Sibling operation parameters/locals are independent.

Use explicit `mail.send(...)` for calls to the enclosing service, just as for
other services. A parameter named `mail` shadows that receiver. This decision
does not introduce implicit `self`, `this`, or bare operation-name lookup. Bare
function calls remain outside the service member-call checker until ordinary
application function resolution is implemented; they are not validated by this
slice and must not be advertised as such.

```sovra
service mail {
    fn send(message: String);
    fn relay(message: String) {
        mail.send(message)
    }
    fn example(mail: Client) {
        mail.send("local receiver")
    }
}
```

Under option A, relay's call resolves to the service and checks operation/arity.
The example parameter shadows the service, so its member call remains an ordinary
object reference outside service validation. `Client` is unresolved application
annotation text here, not an implemented source type.

Operation bodies enter the same 128-depth structural guards. Unsupported syntax
rejects the whole file; it cannot yield partial successful call records. Direct
Rust callers still supply visible identities explicitly. Project checks must
continue validating bindings/imports before invoking service inspection.

## Metadata and compatibility

Retain file-relative declaration/member spans and identify operation records by
qualified enclosing name such as `mail.relay` in the existing function-record
surface. Ordinary function identifiers cannot contain a dot. `is_task` is false;
operation identity is metadata, not execution or exported package identity.
Document the qualification in JSON so consumers do not interpret it as a new
source identifier spelling. Public compiler-stage APIs remain intact.

Previously unsupported bodies may now pass opt-in structural inspection or report
E4093/E4094 for real service errors instead of E4096. Ordinary project wiring
checks remain partial. No signature-type resolution, overloads, implicit service
values, network adapters, automatic exports or invocation lifecycle is selected.

## Alternative B: implicit enclosing-service receiver

Introduce `self` or bare sibling-operation lookup within an operation body.
This needs additional syntax/binding precedence, collision, escaping and value
semantics. Those rules are not implied by ADR 0006 and complicate the current
partial checker. Defer this alternative unless explicitly desired.

## Implementation and acceptance

Parse service operation signatures/bodies into scoped records without substring
matching. Test same-service and imported-service calls, shadowing before/after
initializers, sibling isolation, missing operations and arity, nested arguments,
comments/strings, mixed declaration-only/implemented operations, excessive depth,
source locations and CLI JSON coverage. Preserve empty contracts and unsupported
application constructs. Mark M12 incomplete until project typing and the remaining
declaration/body grammar are implemented and verified.

## Approval boundary

AGENTS.md requires: “Document and pause before fundamental syntax, memory-model,
type-semantics or compatibility decisions.” Enclosing-service visibility and
qualified operation metadata require this explicit decision. The instruction to
finish the roadmap authorizes implementation work but does not select new binding
semantics implicitly. Option A is recommended for a small, predictable rule set.
