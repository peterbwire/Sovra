# ADR 0003: Reject exact builtin callable-name collisions

Status: Accepted and implemented. User approved Option 1 on 2026-09-21.

## Context

Semantic call lookup and both runtimes prioritize builtin functions over user
declarations. Previously, top-level `fn print` and exported `std::len` could pass
declaration checking while calls always reached the builtin. Their user bodies
were unreachable through those names. The bare print compatibility alias must
remain supported.

## Decision

Reject exact callable-name collisions with the builtin registry using E3016 at
the conflicting function declaration. Check every top-level function name and
every qualified module function name, even when unused. ADR 0005 extends
this rule to callable private helpers. This includes
bare `print`, `std::print`, `std::println`, `std::len` and `std::to_string` today.
The registry is authoritative rather than a second hard-coded reservation list.

Noncolliding user functions in `std` remain permitted. ADR 0005 supersedes
the original private-function exemption: private builtin collisions now fail. Unrelated names such as top-level `len` and
`other::print` remain valid. Existing builtin calls and bare print calls retain
their behavior.

## Compatibility and alternatives

This rejects previously accepted declarations. Rename a conflicting user
function or place it under a noncolliding module. Reserving the entire `std`
module was considered and declined in favor of the narrower change. Keeping
silent builtin precedence for colliding declarations was also declined.
Future additions to the builtin registry can introduce new exact collisions;
review that compatibility impact with any stdlib expansion.

Function values, private-function execution and module-local lookup are not
introduced by this decision. Duplicate declarations within one inline module
independently produce E3008 regardless of export visibility.
