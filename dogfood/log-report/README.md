# Log Report CLI

Status: **Implemented bounded slice**. This command reads UTF-8 stdin lines
in `INFO|message`, `WARN|message` or `ERROR|message` form, counts levels and
reports malformed lines with a one-based line number and status 1. Messages
must be nonempty and may contain additional `|` characters. Input is bounded
by the existing one MiB line limit and 10,000 lines. It summarizes a stream;
it does not persist or search logs.

```text
cargo run --locked -- run dogfood/log-report/main.svr < dogfood/log-report/sample.stdin
node scripts/test-dogfood.mjs
```

The shared dogfood suite checks source, IR, interpreter and generated
JavaScript behavior against reviewed success and failure transcripts.
