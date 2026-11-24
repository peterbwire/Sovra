# Standalone generic source execution

`compile_generic_source(source, entry, GenericSourceLimits)` collects a complete
standalone source file and returns sealed `CompiledGenericSource` only after all
admitted declarations, signatures and symbolic bodies pass checking. It then runs
the selected root through specialization, concrete rechecking, literal decoding,
logical layouts, nominal operation planning and typed instruction validation.
`execute_generic_source(&program, GenericVmPolicy)` executes that IR using the
[host-free Rust VM](generic-runtime-v2.md).

This additive Rust library API admits generic aliases, records and enums, plus
generic and nongeneric root functions. Declaration order permits forward references.
Record, enum and function exports retain their original source spelling. Generic
callees require explicit nonempty `::<...>` arguments. Ordinary nongeneric calls
use `helper(...)`, including recursion and calls from generic functions. They
share existing call/signature/type/work admission without retaining a written
type-argument list. Active value bindings still shadow callable names.

The selected entry must exist and have no type or value parameters. Its result
may be any supported type; this API does not impose command-line exit semantics.
All function bodies and type definitions are checked, including unused ones.
Only reachable concrete instances are specialized and lowered. No successful
partial program or fallback is returned after a refusal.

The collector uses original lexer tokens to delimit declarations, so String and
line-comment braces cannot terminate a body. Each declaration parser receives an
unchanged original slice. Whole-file source and exact declaration origins remain
owned by the result. Primary compile/runtime error spans map to original file
bytes and zero-based line/character columns when ownership is known; nested
stage errors retain their fragment or alias provenance. Some global limit errors
have no primary span. `file_span(instance, local)` validates byte boundaries and
recomputes coordinates rather than trusting supplied local line/column fields.
Source/result inspection is trusted host work outside VM metering.

`GenericSourceSyntaxLimits` bounds whole-file frontend input, declaration count,
per-declaration bytes, aggregate retained bytes, tokens, names, parameters,
members, annotation nodes/edges/uses, entry bytes, brace depth and collection work.
Default admission allows 4 MiB original source, 512 declarations, 128 KiB per
declaration and depth 128. Retained bytes include the original file and parser
fragments; token accounting includes the full-file pass and each declaration
pass with its EOF. Annotation lexers retain their separate bounded profiles.
Every existing semantic stage receives one cumulative whole-program profile;
none is reset for each declaration. Runtime resource/control policy remains a
separate explicit argument. These logical limits do not establish a physical
heap bound or process sandbox.

Imports, modules, application declarations, plain type declarations, exported
aliases, standard function/host calls and unknown trailing syntax are explicitly refused
in this profile. [Explicit generic CLI check/run](generic-cli-v2.md) now use this
pipeline for standalone files. Existing Program and other command modes remain
separate. [Host-native source images](generic-native-v2.md) now package checked
generic execution. [Root package commands](generic-packages-v2.md) compile guarded
captured entry bytes. Dependency/module linking, JavaScript emission, tooling integration,
Adapters and postfix propagation remain open. This slice does not complete
G13 E2 or G13.

`tests/generic_source_v2.rs` covers mixed complete files, forward references,
ordinary/generic recursion, strict call rules, unused-body refusal, original
source/error mapping, Unicode boundaries, cumulative quota boundaries, unsupported
contexts, runtime exhaustion and every UTF-8 prefix of a small source fixture.

[Qualified standard Option/Result](generic-standard-v2.md) now use compiler-owned
nominal definitions, separate user provenance and the existing checked enum pipeline.

[Structured std::Error](generic-error-v2.md) now supplies checked code/message/
optional-offset fields through the compiler-owned nongeneric record profile.
