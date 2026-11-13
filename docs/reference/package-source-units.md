# Source-owned registered package units

Status: **Partial G12; additive Rust API, Linux guarded capture implemented.**
`packages::load_package_sources(root)` builds on the approved
[source inventory](package-source-inventory.md), captures selected file bytes and
parses each original file independently. It does not yet check semantic interfaces,
resolve import visibility, link IR, execute external modules or implement locks.
Older Rust/context/overlay/formatter modes remain strict; executable CLI commands
now route registered graphs to the source-owned compiler.

`LoadedPackageSources` exposes read-only accessors:

- `inventory()` retains the canonical graph and registration metadata.
- `manifests()` retains read-only `PackageManifestSnapshot` values: canonical root/path, exact parsed bytes, declared dependency locators and raw-byte SHA-256.
- `units()` returns source-owned `PackageSourceUnit` records in inventory order.
- `token_count()` and `ast_nodes()` report admitted cumulative counts.

Each unit has `source_index()`, `source()`, `program()` and `imports()`. The source
index belongs to the inventory table; its owning package/path identify every
original byte/scalar span. Source text and AST are retained together without
mutable external access, concatenation, synthetic wrappers or foreign-span grafts.
Changing a disk source after loading does not change the returned source/AST.
These are parsed units, not checked units or permission to execute unvalidated IR.

Registered module files require exactly one matching existing inline `mod` plus
optional existing top-level dependency `use alias::module;` imports. Other top-level
functions/types/records/modules are rejected. Entry syntax is unchanged. Module
names across entry/registered files must be unique within each owning package;
equal names in different packages do not collide. Imports retain exact original
spans and file ownership and are not granted to sibling sources. Import targets,
visibility, alias/record types, bodies and returns are not semantically validated
here; syntactically valid bad types or undeclared imports can therefore load.

Lexical/parser errors keep existing codes and owning source paths. E4162 reports
registered header/shape mismatches. E3008 reports duplicate package module names.
E4161 reports capture/UTF-8 failures or unavailable guarded capture. I/O/admission
errors use file-level zero spans; actual parse/header errors retain original spans.
No partial unit collection is returned on any failure.

The current reader requires Linux x86/x86_64/aarch64 and readable procfs descriptor
links. It opens with nonblocking/no-follow flags, validates regular-file device/
inode/size/change metadata, and attests the opened descriptor's exact canonical
location and package-root containment before reading. It checks the opened handle
and location again after bounded reads, and rejects repeated opened identities.
This prevents tested link/FIFO/regular-file/parent-link substitution cases from
being mistaken for the selected source. Unsupported hosts/procfs availability fail
explicitly. Linux tests do not qualify Windows/macOS or other architectures.

Retained UTF-8 bytes are exactly those passed to lexing/parsing; there is no second
source read for the AST. Limits cover actual bytes, not inventory size estimates:
16 MiB per source, 64 MiB total, 1,000,000 tokens per source, 4,000,000 original
non-EOF tokens total and 1,000,000 aggregate AST nodes, plus existing per-source
parser/depth/diagnostic limits. Inventory manifest/package/source-count limits
still apply. Excess bytes/tokens report E1003; AST admission retains E3020. All
limits reject before a partial successful collection is exposed.

Manifests are now captured by the same guarded reader before graph parsing, within
existing manifest budgets. Their retained bytes are exactly those parsed for the
graph. [Source locks](package-source-locks.md) hash these snapshots and source
buffers; locked compilation consumes those same owned inputs. This is not an
atomic snapshot of a changing filesystem or an immutable directory capability.


The next additive [registered callable compiler](registered-package-functions.md)
now consumes these units through per-source checked semantics/lowering and links
primitive function interfaces. It retains source ownership and does not reopen
source paths. Record-capable compilation, executable CLI routing and explicit locks now exist; complete context/editor integration remains open; this loader alone still performs syntax admission only.
