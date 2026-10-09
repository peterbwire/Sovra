# Package consumer

This runnable application consumes `library/` through a direct local dependency
declared in `app/sovra.toml`. The library exports nominal `Entry` and `Ledger`
records and operations from `budget`; its `describe` helper remains private.
The application constructs records, reads fields, totals a three-entry ledger,
and builds a revised immutable ledger when passed `revised`.

From the repository root:

```text
cargo run --locked -- check dogfood/05-package-consumer/app
cargo run --locked -- run dogfood/05-package-consumer/app
cargo run --locked -- run dogfood/05-package-consumer/app -- revised
node scripts/test-dogfood.mjs
```

The dogfood suite checks and builds the package, then compares both Rust
interpreter and generated JavaScript output with the two reviewed transcripts.
This exercises local package linking. Registry publishing, installation,
version resolution, and lockfiles are not implemented.
