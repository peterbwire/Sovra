# ADR 0039: Cumulative interface string materialization

Status: **Accepted and implemented.** User approved the 128 MiB default.

## Implemented

`packages::compile_with_interface_string_limit(root, max_bytes)` checks cumulative
UTF-8 payload bytes before retaining interface string copies. Existing source,
manifest, AST, interface-node and linked IR defaults still apply. Ordinary package
APIs and their executable CLI callers enforce the approved byte ceiling below.

One budget spans all canonical packages in a compilation. Count each owned
occurrence when materializing:

- Normalized imported/export-validated signature names, parameter names and
  present canonical parameter/return annotations. Function bodies are excluded.
- Canonical exported-record cache names, field names and normalized field types.
- Record attachments to consumers: record names and all field names/types.
- Generated type-alias AST names and canonical targets, on first spelling only.
- Qualified visible-signature labels and their copies into the signature-reference
  vector. Overwritten/repeated imports consume their materialization allowance
  again; only the final visible labels are copied into that vector.

Canonical names count their actual UTF-8 length, including generated package
prefixes. Long aliases that normalize to Int count the three copied bytes of
Int, rather than the annotation spelling. A byte is not refunded when temporary
validation output or overwritten metadata is dropped. Empty text consumes zero.
An empty metadata interface can compile with a zero allowance.

E3024 rejects exhaustion with no successful partial linked program. Import
normalization failures caused by this quota point to the consuming import;
ordinary type errors retain owner-source context. Exported-cache copies of
attached records also point to their import, rather than foreign declaration
offsets paired with the consumer's file.

Signature normalization builds parameter metadata directly, avoiding a clone
of source annotation strings before canonical replacement. Exported-record
normalization now copies fields directly and no longer constructs an intermediate
synthetic function signature. Type resolution rules and successful IR remain.
Public PackageCompilationLimits and existing API signatures are preserved.

## Approved default

**128 MiB cumulative interface string payload per executable package graph.**
Ordinary executable package compiler APIs and check/run/build/test paths using
them enforce it. compile_with_compilation_limits, compile_with_source_limit and
compile_with_graph_limits retain this default. The explicit byte-limit API honors
caller-selected allowances, with existing non-string defaults retained.

Measured without executing source:

| Graph | Interface string materialization |
| --- | ---: |
| examples/local-packages/app | 464 bytes |
| examples/local-packages/utilities | 117 bytes |

These small fixtures do not demonstrate production-scale performance or a safe
process memory ceiling. Profiles are retained in target/fuzz-evidence/.

## Verification and boundaries

Tests independently sum a two-alias record/signature graph and reject every
smaller byte allowance. They cover exact/zero limits, repeated imports, generated
canonical names, long annotation aliases, correct file/range attribution,
invocation independence, UTF-8 byte accounting and usize overflow prevention.
The full suite retains package execution, widening and semantic rejection checks.

This quota counts interface payload materialization, not all resident memory.
It excludes parsed/prepared/typed AST and semantic-stage copies, type-resolution
temporaries, lookup/target/record-identity maps, allocator capacity/overhead,
source-local names, IR strings, backend output and runtime data. A formatted alias
or canonical-resolution temporary may already exist before its copy is charged.
The compiler still has no complete memory/CPU budget or execution isolation.

## Decision boundary

[AGENTS.md](../../AGENTS.md) requires: “Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions.” Enabling a new
default rejects previously accepted valid graphs. The concrete mechanism was
implemented and tested before review; the user explicitly approved activation.
Default-boundary tests admit exactly 128 MiB, reject overflow through all ordinary
package APIs and five CLI paths, and accept a larger explicit byte allowance.
