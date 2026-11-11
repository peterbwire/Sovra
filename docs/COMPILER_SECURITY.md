# Compiler security

Security is a continuing engineering requirement, not a claim that Sovra is the
most secure language. No independent security audit or comparative certification
has been completed. This document separates current safeguards from open work.

## Threat model

Treat source text, source-derived names and literals, local dependency contents,
and serialized IR as potentially hostile inputs. Protect the compiler process,
generated code integrity, diagnostic accuracy and existing output files.
Malformed input should fail with diagnostics instead of causing a panic, stack
overflow, unintended code execution or silent acceptance.

Checking and emitting IR/JavaScript must not execute Sovra source. Running source
or tests executes its file and process-I/O effects with the host user's authority;
these commands are not a sandbox. Native executable linking invokes a trusted
Rust/Cargo/linker toolchain. Cargo configuration, environment variables, executable
search paths and runtime source are part of that trusted build environment.
Payload hashes detect corruption; they do not authenticate a maliciously replaced
payload and matching manifest.

## Existing safeguards

- Rust implementation forbids unsafe code. Source type checking precedes normal
  lowering and execution; explicit public low-level APIs have separate contracts.
- Checked Int arithmetic and array bounds checks reject invalid operations.
  Interpreter and JavaScript behavior have differential regression tests.
- Source delimiter, expression and statement nesting is bounded before building
  excessive recursive trees. Caller-built ASTs receive a borrowed iterative depth
  preflight at semantic-analysis entry. `else` chains share the statement budget
  with nested blocks, even when few braces are open.
- Generated JavaScript escapes strings, parses numeric literals before rendering
  them and uses null-prototype maps for source-controlled function/field names.
- Native IR decoding bounds image size and counts, checks lengths and UTF-8,
  and rejects unknown instruction forms, invalid numeric literal payloads and
  invalid call/declaration metadata, including in unreachable instructions.
- Native output publication refuses overwrites; installed runtime payload checks
  reject missing/extra/modified files, links and excessive structural sizes.
- Default executable frontends enforce 16 MiB/source, 1,000,000 non-EOF tokens
  and 256 lexer/parser diagnostics, rejecting exhaustion without partial success.
  CLI source compilation, language tests and package entries use bounded reads
  of at most the byte ceiling plus one detection byte. The formatter retains
  its stricter 1 MiB read limit. Rust callers may select explicit FrontendLimits;
  see [ADR 0035](adr/0035-frontend-resource-budgets.md) for coverage and limitations.
- Executable package compilation defaults to 64 MiB total entry sources across
  canonical packages, including the root. Reads honor the remaining allowance
  before allocation; aliases/shared dependencies count once. Trusted Rust hosts
  may use `packages::compile_with_source_limit` for explicit aggregate policies.
- Default graph loading enforces 1 MiB per manifest, 8 MiB total canonical
  manifests and 8,192 alias edges. Initial project classification and ordinary
  project manifest reads also use the per-file cap. Trusted Rust hosts may
  select explicit PackageGraphLimits. See accepted
  [ADR 0036](adr/0036-package-graph-resource-budgets.md) for accounting and scope.
- Malformed-manifest parsing and executable semantic analysis retain at most
  256 errors, including a stopping notice (E4018/E3019). Production is bounded
  as errors are emitted, rather than trimming an already oversized report.
  Malformed classification aborts early, and exhausted semantic traversal skips
  later validation without returning successful partial output.
- Guarded compiler messages retain at most 4 KiB each, preserving UTF-8 and
  structured error metadata with a visible truncation suffix. Oversized backing
  allocations are replaced, not merely shortened. Transient formatting, file
  identity, JSON escaping and raw caller-owned records are outside this ceiling.
- Rust hosts can opt into borrowed AST admission and retained IR instruction/
  string budgets. Emission is charged before retention and partial failures return
  no successful IR; [ADR 0037](adr/0037-source-unit-admission-and-ir-budgets.md)
  defines approved standalone defaults: 250,000 AST nodes, 1,000,000 retained IR
  instructions and 64 MiB retained IR string payload. Check enforces AST admission;
  standalone run/build/test additionally enforce emission quotas. Explicit callers
  can select limits; raw trusted lowering remains separate and package linking
  uses the graph policy below.
- Package hosts can select cumulative source AST, interface metadata and final
  linked IR quotas through `compile_with_compilation_limits`. The emitter charges
  qualified names before retention; interface imports copy signature metadata
  without cloning bodies. [ADR 0038](adr/0038-package-compilation-budgets.md)
  defines approved graph defaults: 1,000,000 AST nodes, 250,000 interface nodes,
  4,000,000 linked instructions and 256 MiB retained linked string payload.
  Package compiler APIs and executable check/run/build/test enforce them;
  interface string bytes, temporary storage and repeated semantic work are
  outside these quotas.

These are individual safeguards, not proof that every input or execution path is
safe. The source compiler does not impose a complete memory or CPU budget.

## Required verification

Every confirmed defect needs a failing regression before its fix. Run formatting,
strict Clippy and the full Rust suite. For structural changes also run
`node scripts/probe-compiler-depth.mjs --assert-limits` and
`node scripts/test-compiler-security.mjs`; both isolate compiler invocations with
timeouts. Preserve original source ranges and stable diagnostic codes. Successful
packaging or compilation alone does not establish security qualification.

`cargo test --test compiler_security` runs 4,096 deterministic source mutations
and 4,096 binary-image mutations, plus truncation/trailing-data checks. Successful
source mutations pass through analysis/lowering and IR round-tripping; rejected
ones retain valid diagnostic ranges. Mutated programs are never executed. CI
gives this regression target a two-minute process budget. These fixed-seed checks
are reproducible corpus tests, not sustained coverage-guided fuzzing.

Isolated coverage-guided source and native-image targets are available under
[fuzz/](../fuzz/README.md), with reviewed source seeds and generated image corpora.
They use libFuzzer and AddressSanitizer and never execute the generated programs.
Keep failure artifacts private until triaged. Bounded campaign results do not
establish exhaustive coverage; input limits are specific to the test harness.

## Open security work

1. Bound remaining valid-input semantic/IR work and transient diagnostic formatting.
   Per-file frontend limits do not establish an end-to-end memory or CPU quota;
   scanner-only project paths need separate resource coverage.
2. Run sustained coverage-guided campaigns against the source/native-image targets,
   expand seed diversity and input sizes, and retain minimized regression corpora.
   Short local campaigns and deterministic probes do not replace sustained fuzzing.
3. Audit native build configuration inheritance, dependency filesystem races and
   runtime-source provenance. Do not use native linking in an untrusted build
   environment as an isolation boundary.
4. Design execution isolation and resource/capability policy separately before
   offering untrusted-code hosting; current run/test commands provide neither.
5. Qualify supported hosts/MSRV, reproducible artifacts and dependency provenance,
   and commission independent review before making broad security claims.

Report vulnerabilities according to [the security policy](../SECURITY.md).

When extending executable semantic validation, use the scoped push_semantic
helper; manifest syntax errors use push_manifest_error. The public raw Diagnostics
container remains available and is not globally capped. Compiler entry-point
guarantees do not cover arbitrary caller mutations of that raw collection.
