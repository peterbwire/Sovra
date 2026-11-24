# Checked generic call navigation (V2)

Optional generic `--symbols` evidence now includes direct user-function calls.
Each reference carries the original callee identifier range and its declaration
index in the same complete source envelope. The target comes from the checked
caller/callee binding identity; the name token is retained during symbolic body
checking. No compiler or editor name search infers a binding.

Projection visits each original checked symbolic body once, including unused
functions and unreachable code. Multiple concrete specializations do not duplicate
source references. Generic, ordinary, recursive and grouped ordinary calls retain
exact identifier spans. Type declarations and functions with the same spelling
remain separate identities. Strings/comments, record/enum constructors and host-free
builtin calls do not become user-function references. Active value shadowing still
refuses an invalid root function call during checking.

`GenericSymbolLimits.max_references` independently admits the whole reference batch,
with a hard ceiling of 8192. Every body/fact visit, source name check and emitted
reference consumes shared projection work/bytes. Caller ownership, target table
ownership, source declaration mapping and exact original name ranges are verified.
Any refusal discards all optional facts, preserving the successful check outcome.

Generic editor mode now advertises Go to Definition for these calls and displays
the target's checked declaration header on hover. Signatures remain declaration
signatures, not reconstructed concrete substitutions. Unicode/CRLF offsets use the
existing UTF-16 conversion. Invalid or unavailable snapshots retire old facts.
Declaration outlines/hover and standalone Check/Run remain available. Type-use
references outside function headers, formatting, generic package overlays and semantic completion remain open.

Tests cover recursive/unused/grouped calls, repeated specialization, type/function
name collisions, shadowing and non-code text, reference quotas, compiler JSON,
unsaved LSP buffers and actual VS Code call definition/hover. No native format or
runtime behavior changed. Native execution, packaged VSIX installation and
other-host editor execution were not requalified. Broader E2/G13 work remains open;
Task Manager is unchanged.

See [checked generic local navigation](generic-local-navigation-v2.md) for parameter/local/payload definitions and checked type hover.

See [checked function-header type navigation](generic-type-navigation-v2.md) for record/enum/alias annotation definitions and hover; other type contexts remain open.
