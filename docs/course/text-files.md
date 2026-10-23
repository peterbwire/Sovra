# Reading and replacing text files

Sovra programs can read bounded UTF-8 files and replace them with the approved
ADR 0015 standard-library functions. Both return records so a missing file or
failed write can be handled in the program.

```svr
fn main() {
    let loaded: std::TextRead = std::read_text("notes.txt");
    if (loaded.ok == false) {
        print("read failed: " + loaded.error);
        return;
    }

    let saved: std::TextWrite = std::write_text("notes-copy.txt", loaded.text);
    if (saved.ok == false) {
        print("write failed: " + saved.error);
        return;
    }
    print("copied " + std::to_string(std::len(loaded.text)) + " bytes");
}
```

Relative paths use the process working directory, not the source file's
directory. Files must be valid UTF-8 and at most 16 MiB. A successful empty
file has `ok: true` and empty `text`; a failed read has `ok: false`, empty
`text`, and an error category. Categories are `not_found`,
`permission_denied`, `invalid_path`, `invalid_utf8`, `too_large`, and `io`.

`std::write_text` creates a temporary file beside the destination, syncs it,
then renames it over the destination. A handled failure before rename preserves
the old file. The API does not create parent directories or promise durability
after sudden power loss. It does not provide append, binary data, handles or
transactions.
On Unix, new and replaced files request owner-only `0600` mode, subject to a
more restrictive umask. Replacing a group-shared file narrows its mode. On
Windows, use a private directory for sensitive text because custom ACLs are
not preserved by this API. A write to a symlink path can replace the link;
concurrent writers need a separate coordination mechanism.

The runnable [File Processor](../../dogfood/02-file-processor/README.md) has a
`copy SOURCE DESTINATION` mode. From the repository root, use:

```text
cargo run --locked -- run dogfood/02-file-processor/main.svr -- copy input.txt output.txt
```

The interpreter and generated JavaScript receive the same source API. Run
`node scripts/test-text-files.mjs` after building to exercise both engines.

## Rejecting stale updates

Approved ADR 0034 adds conditional replacement for cooperating writers:

```svr
fn main() {
    let loaded = std::read_text("notes.txt");
    if loaded.ok == false && loaded.error != "not_found" {
        print("read failed: " + loaded.error);
        return;
    }
    let saved: std::TextWrite = std::write_text_if(
        "notes.txt", loaded.ok, loaded.text, loaded.text + "New note\n");
    if saved.ok == false { print("not saved: " + saved.error); return; }
    print("saved");
}
```

The comparison uses the original content, including a UTF-8 BOM if present.
`conflict` means another conditional writer changed existence or content; reload
before asking the user to retry. `busy` means a coordination marker already
exists. Neither result authorizes automatically replaying a mutation.

Conditional writes use a sibling `.svr-write-lock` marker, not a public file
handle. A crash or cleanup failure can leave it behind. Stop **all** writers,
inspect the data, then deliberately remove a retained marker to recover. Never
remove one based only on its age or while another writer might still run.
Successful commit is not undone by marker-cleanup failure. Keep files in a
private local directory, do not write internal marker paths, and keep old apps,
unconditional `std::write_text` calls and external editors from writing the same
state concurrently. Symlink destinations are rejected by the conditional API.
Different files are independent; several conditional writes are not a transaction.

Run `node scripts/test-conditional-text-files.mjs` for two-engine behavior,
Task Manager rejection paths and deterministic mixed-engine conflict checks.
