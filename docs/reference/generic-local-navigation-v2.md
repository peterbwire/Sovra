# Checked generic local navigation (V2)

Optional standalone generic `--symbols` evidence includes parameters, immutable
and mutable locals, and match payload bindings. Reads and assignment receivers
retain original identifier tokens and checked body-owned binding identities.
Initializer lookup precedes declaration, nested scopes preserve shadowing, and
indexed writes point at the receiver name. Projection visits original symbolic
bodies once, including unused functions; specialization does not duplicate facts.

Generic editor mode supports Go to Definition and checked type hover for these
bindings. Hover labels include roles and checked types, such as `parameter x: T`,
`let x: [Int]`, `let mut x: std::Option<T>`, or `match payload x: T`.
Types are normalized from retained identities: aliases expand, arrays use `[T]`,
and nominal arguments retain declaration order. Symbolic `T` is not replaced by
a call specialization. Labels do not claim that every local was inferred. Local
outline entries, type-use navigation outside function headers, formatting and generic package overlays
remain open. Existing Unicode/CRLF byte-to-UTF-16 conversion applies.

Retained uses share a whole-body quota derived from admitted expression and
statement counts and consume body work. Projection independently caps locals and
local references at 8192 each, checks binding ownership and original source
ranges, and shares existing output-byte/string/work limits. Exhaustion discards
all optional facts without failing compilation. Invalid edits retire old facts.

This slice changes editor evidence, without changing execution or the native
feature contract. E2/G13 remain incomplete. Task Manager is unchanged.

Type rendering uses an iterative task stack, with string and temporary task
admission bounded by the existing 4096-byte per-string ceiling. Each node/edge
visit and emitted type byte consumes shared projection work. Shared DAGs may
expand when displayed; repeated expansion remains bounded by those quotas.
An oversized label refuses the entire optional envelope rather than truncating
its type or keeping earlier declarations. No new runtime or native feature is
introduced. Function-header type-use navigation is now available; other type-use contexts remain open.

See [checked function-header type navigation](generic-type-navigation-v2.md) for record/enum/alias annotation definitions and hover; other type contexts remain open.
