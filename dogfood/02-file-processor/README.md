# File processor

This first File Processor slice reads UTF-8 lines from standard input, numbers
them, optionally drops empty lines, and reports counts. It uses `std::read_line`,
`std::len`, process arguments and print from the approved Stage 1 API. `std::len`
counts UTF-8 bytes, so the summary reports bytes excluding line terminators.

From the repository root, after building the local compiler:

```text
cargo run --locked -- run dogfood/02-file-processor/main.svr
cargo run --locked -- run dogfood/02-file-processor/main.svr -- nonempty
cargo run --locked -- run dogfood/02-file-processor/main.svr -- copy input.txt output.txt
cargo run --locked -- run dogfood/02-file-processor/main.svr -- select input.txt output.txt '::'
node scripts/test-dogfood.mjs
node scripts/test-text-files.mjs
```

The suite supplies input fixtures and verifies both interpreter and generated
JavaScript output for all lines, dropped empty lines, and immediate EOF. Input
is bounded by the existing one MiB line limit. Copy mode reads a UTF-8 source
file and replaces a destination with the general ADR 0015 API. A separate
cross-backend subprocess suite verifies Unicode, empty files, existing-file
replacement, invalid paths and encodings, missing files and size limits.
`select` extracts the text before the first exact delimiter on each line and
replaces the destination only after every line validates. A missing delimiter
reports its one-based line number and leaves the destination intact. It preserves
whether the final line has a newline, supports Unicode and multicharacter
delimiters, and rejects an empty delimiter. The transformation accepts at most
1 MiB and 10,000 lines; copy mode retains the underlying 16 MiB file bound.
This is a bounded text-processing slice, not a binary-file API.
