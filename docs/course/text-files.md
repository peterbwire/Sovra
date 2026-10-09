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

The runnable [File Processor](../../dogfood/02-file-processor/README.md) has a
`copy SOURCE DESTINATION` mode. From the repository root, use:

```text
cargo run --locked -- run dogfood/02-file-processor/main.svr -- copy input.txt output.txt
```

The interpreter and generated JavaScript receive the same source API. Run
`node scripts/test-text-files.mjs` after building to exercise both engines.
