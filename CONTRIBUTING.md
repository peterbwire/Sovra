# Contributing

1. Install the stable Rust toolchain with `rustup`.
2. Make focused changes and update documentation when a public contract
   changes.
3. Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features
   -- -D warnings`, and `cargo test` before opening a pull request.
4. Keep M0 free of lexer/token behavior; that work starts in M1.

Please include a concise description, validation commands, and any compatibility
considerations in pull requests. See [SECURITY.md](SECURITY.md) for security
reports.

