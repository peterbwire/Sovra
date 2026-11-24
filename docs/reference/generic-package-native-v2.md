# Source-backed generic package native executables

Generic native Build now supports registered modules and local dependency graphs:

```sh
svr lock --experimental-generics ./project
svr build --experimental-generics --locked --emit exe --output ./application ./project
```

On the qualified Linux host, the executable runs without Node.js, Cargo/Rust at
startup, original project directories, manifests or lock files. Rust/Cargo and a
host linker are build-time requirements. This is a Rust executable embedding
Sovra source plus compiler/VM execution, not direct Sovra machine-code generation.
The root entry must be nongeneric, parameterless Unit main. Arguments and I/O hosts
remain unavailable. Unexpected arguments refuse before program execution.

Build captures/locks and checks the complete graph before output construction.
Stale locks and invalid source refuse without creating output parents. Existing
output refusal, runtime SDK integrity checks, staged linking, publication and
cleanup reuse the existing native builder. Single-entry source/package builds keep
the previous SVRG2 family and feature words; registered graphs use the distinct
SVRGP family. No old decoder is relaxed to accept new package metadata.

The package image contains only dependency aliases/targets, source ownership,
registered module names, normalized package-relative source locators and original
UTF-8 source bytes. It omits absolute project roots, manifests, lock contents,
arguments, environment values and grants. Startup validates all framing/topology,
then rechecks every source through the source-aware generic compiler and executes
checked IR under the existing finite host-free command policy. It never loads
project sources or a dependency from the host filesystem.

`GenericPackageSources` now distinguishes guarded capture from validated embedded
sources. Its inventory paths are canonical for captures and logical for embedded
sources. `captured()` returns Some only for a real guarded capture and None for an
image; embedded manifests are empty. Image bytes cannot construct a
`CapturedPackageSources` accepted by filesystem lock readers. Diagnostics use
logical `@sovra-image/pN/path.svr` locations (host path separators may vary), not
original filesystem authority. The source/import/nominal ownership model is
preserved when rechecking aliases, signatures, bodies, matches and specialization.

Version-one framing is 16 bytes: six-byte `SVRGP\0` magic, one-byte version 1,
one-byte value model 2, little-endian u32 exact feature word 511, then little-endian
u32 payload length. The payload starts with u32 package count. Each package has
u32 edge count followed by sorted unique (length-prefixed alias, u32 target) edges.
Then comes u32 source count; each source has u32 owner, u8 kind (0 entry, 1 module),
module name only for kind 1, relative path and source text. All strings use u32
UTF-8 byte lengths. Unknown versions/models/features/kinds, malformed UTF-8,
truncation, trailing bytes, cycles, unreachable packages, duplicate aliases/paths,
missing entries, unsorted owners/modules and unsafe path spellings refuse.

Native-specific ceilings are 64 packages, 256 sources, 512 dependency edges,
64 KiB aggregate metadata strings, 4096 bytes per metadata string (1024 for
identifier aliases/modules), 1 MiB aggregate source and 2 MiB complete image.
Source count/length admission precedes materialization; parsing first retains
borrowed bounded wire slices and constructs owned source records only after
structural validation. Compiler stage quotas remain independently active, so a
graph inside image limits can still refuse semantic/resource checks. Limits are
logical admission, not signing/authentication, OS isolation or hard deadlines.

Tests cover exact/exceeded package/source/source-byte caps, malformed topology,
header/profile refusal, every truncation, byte-corruption panic resistance,
old/new family separation, semantic/runtime rechecking and embedded-origin
capability separation. An independent locked native consumer uses exported generic
records/helpers and propagation; after building, its project is deleted and the
executable runs from another directory with empty environment and nonexistent
PATH. Conflicting files at logical image locations are ignored. No other-host,
release-signing, full-backend or production-readiness claim follows from this
Linux result. E1/E2/G13 and broader qualification remain open. Task Manager and
editor behavior are unchanged.

The separate [collection image profile](generic-collection-native-images-v2.md)
now uses exact word 1023 through explicit library APIs. This page's existing
word-511 encoder/decoder and CLI/native path remain strict and unchanged.
