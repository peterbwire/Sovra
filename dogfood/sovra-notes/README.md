# Sovra Notes

Status: **Partial persistent CLI**. `list`, `get`, `put` and `delete` operate on
a bounded versioned UTF-8 file. Malformed files are rejected before writes.
The app uses a separate local package under `library/` that exports nominal
`Note`/`Parsed` records and validation, encoding, parsing, `empty_store`, and
`valid_store` functions. Other
local Sovra packages can import `notelib::notes` through a manifest path.

```text
cargo run --locked -- run dogfood/sovra-notes/app -- put notes.txt greeting hello
cargo run --locked -- run dogfood/sovra-notes/app -- get notes.txt greeting
node scripts/test-notes.mjs
cargo run --locked -- test dogfood/sovra-notes/tests
```

The separate test package consumes this reusable library and checks key
validation, nominal note encoding and malformed/valid record parsing.

Keys are 1–80 UTF-8 bytes; values are at most 4,096 bytes. Both exclude `|`,
CR and LF. The file is bounded by `std::read_text` (16 MiB) and 10,000 notes.
This is single-writer storage: concurrent updates may overwrite one another.
The approved atomic replacement API does not provide compare-and-swap or a
transaction. The application does not claim encryption or cross-process locks.
