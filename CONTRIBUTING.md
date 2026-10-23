# Contributing

1. Install the stable Rust toolchain with `rustup` and Node.js (CI uses Node 22)
   for generated JavaScript execution tests.
2. Make focused changes and update documentation when a public contract
   changes.
3. Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features
   -- -D warnings`, and `cargo test` before opening a pull request.
4. Follow `AGENTS.md` and the current development status. Preserve established
   syntax and keep specification, examples, tests and course lessons synchronized.

Please include a concise description, validation commands, and any compatibility
considerations in pull requests. See [SECURITY.md](SECURITY.md) for security
reports.

Keep generated builds, packages, probes and local qualification reports under
`target/`. Keep application data such as the root `tasks.txt` out of source
control; expected transcripts and intentional fixtures belong with their tests.
Before deleting old artifacts, retain the current verified package and its
checksum/report, inspect references, and verify cleanup paths stay within the
workspace. Preserve working compatibility adapters until their replacement is
qualified. An old filename or an unreferenced manual test is not sufficient
evidence that source code is obsolete.

