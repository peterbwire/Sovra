# Versioned collection package images (explicit API)

The collection package compiler now has a separate source-backed image profile
with exact feature word **1023**. Existing package images retain word **511**;
ordinary encoders/decoders remain strict and do not automatically upgrade,
downgrade or fall back between the profiles.

The explicit Rust APIs are:

- `CompiledGenericPackage::encode_generic_collection_package_native_image()`
- `decode_generic_collection_package_native_image(image)`
- `execute_generic_collection_package_native_image(image)`

Encoding requires a program compiled with `compile_generic_collection_package`
and the command entry contract: a parameterless Unit `main`. Ordinary compiled
packages cannot silently promote through the new encoder. The original encoder
still refuses the collection compilation profile, including unused collection
annotations and entry-only programs.

## Framing, ownership and rechecking

The profile uses the existing `SVRGP` package family, version 1 and value model 2,
with unchanged source/topology framing. Only the new API admits exact word 1023.
It refuses word 511, partial/unknown words, other image families, invalid UTF-8,
truncation, trailing data and invalid lengths/topology. The old decoder refuses
word 1023. Changing a collection image's word to 511 does not bypass checking:
the old semantic environment rejects the collection source.

Images contain original registered entry/module source and validated dependency
topology, not trusted machine instructions or caller-provided checked type IDs.
Decoding reconstructs logical package/source ownership and rechecks every body,
including unused bodies, with the collection type/body/layout/IR profile and
`GenericPackageCompilationLimits::DEFAULT`. Key bounds, sealed entry identity,
exact generic arguments and visibility remain checked. Non-Unit/parameterized
command entries and unavailable host operations refuse.

Limits remain shared with ordinary package images: 2 MiB complete image, 1 MiB
aggregate source, 64 packages, 256 sources, 512 dependency edges and bounded
metadata/per-string admission. Embedded sources have memory-only origin and
logical `@sovra-image` locations. They cannot become guarded filesystem captures,
manifest authority, host grants or project-file startup reads. Encoding and
execution work after deleting the original captured project.

Execution uses the existing finite host-free command VM policy, including shared
instruction, materialization and work limits and a cooperative 30-second elapsed
allowance. Exhaustion or runtime failure refuses with no unlimited/backend
fallback. Successful rechecking alone does not imply successful execution.

Tests exercise ordered collections, public entry records, generic module imports,
all six published insertion/lookup/removal/iteration/entry/display examples,
canonical round trips, project deletion, malformed-byte mutation, strict feature
words, source/entry/type errors and runtime resource refusal.

## Remaining gates

[Collection package commands](generic-collection-cli-v2.md) now opt in to this
profile for Check/Run/Test and native Build, including generated runner dispatch.
Actual Linux collection executables run after project deletion without Node/PATH.
The existing word-511 CLI/native path remains unchanged. Broader backend/host/
resource qualification and C1/G13 closure remain open.
