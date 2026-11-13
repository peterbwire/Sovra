# ADR 0040: Primary project scan input quotas

Status: **Accepted and implemented.** User approved the primary-scan defaults.

## Implemented

`project::check_project_with_scan_limits(root, ProjectScanLimits)` bounds the
primary application-wiring scanner's discovery/read inputs. Ordinary
`check_project` and its CLI callers enforce the approved defaults below.
The existing manifest read ceiling remains independent.

- `max_source_bytes`: UTF-8 bytes per discovered `.svr` source path.
- `max_total_source_bytes`: cumulative source bytes across successful reads.
  Every discovered path counts separately, including symlinks to the same file.
- `max_source_files`: discovered source paths retained for scanning.
- `max_directories`: directories scheduled, including root and empty directories.
  Symlinked directories retain existing traversal behavior; no new following or
  containment policy is introduced.
- `max_directory_entries`: yielded entries across traversed directories,
  including non-source files and directory-iterator errors. The manifest itself
  counts as an entry even though its bytes use the separate manifest allowance.

Discovery checks each allowance before retaining an excess path. Reads use the
remaining aggregate allowance and per-file allowance, with at most one additional
detection byte. UTF-8 byte counts include comments/whitespace. Source paths retain
their existing sorted scan order; directory enumeration order is filesystem-led,
so the discovery entry that triggers exhaustion may vary between filesystems.

E4150 rejects exhaustion and identifies the resource/allowance and offending
path in bounded message text. Filesystem admission errors have no invented source
range/file location. No partial successful ProjectCheck is returned and later
wiring/import validation is skipped after stopping.

A failed bounded read may have consumed bytes before returning an I/O/UTF-8
error. Limited scans therefore stop on the first such error (E4009), rather than
reusing an uncharged allowance on later files. Default scans now use this same
fail-fast read-error policy. Zero bytes admit an empty source;
zero directories reject root admission; zero files reject a discovered source.

## Approved primary-scan defaults

- **16 MiB per source path**.
- **64 MiB total source bytes**.
- **4,096 source paths**.
- **4,096 scheduled directories, including root**.
- **100,000 yielded directory entries**.

These apply to primary `check_project` and its CLI callers. This is not
an equivalent quota for `--service-calls`: optional application body/contract
inspection rereads source through separate APIs and remains outside this slice.
Explicit callers can select their own primary scan allowances.

## Verification and limitations

Tests cover exact/overflow limits for all dimensions, zero/empty input, UTF-8
byte counting, ignored-file fan-out, fail-fast byte exhaustion, invalid-read
stopping, symlink read duplication on Unix, invocation independence and exact
valid ProjectCheck parity with prior scanner behavior. Default regressions cover
exact 16 MiB input, 4,096 source paths and 4,096 directories, overflow rejection,
larger explicit byte allowances and human/JSON/--service-calls CLI admission.

These input allowances do not bound aggregate error production, scanner metadata,
alias/type work, source-line/index copies, allocator overhead or runtime execution.
They do not close symlink/filesystem races or impose a new containment policy.
Discovery stores bounded path counts, not a complete path-byte/process-memory
budget. Existing ProjectCheck/public raw inspection structures remain compatible.
No Sovra source executes during the primary check.

## Decision boundary

[AGENTS.md](../../AGENTS.md) requires: “Document and pause before fundamental
syntax, memory-model, type-semantics or compatibility decisions.” Default adoption
rejects some previously accepted valid projects. The opt-in implementation was
complete and tested before review; the user explicitly approved activation.
