# Development

This repository is being rewritten as a single Rust crate that builds the
`tabview` binary.

Run these commands before submitting implementation changes:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

The Python implementation has been removed. The Rust crate is the maintained
implementation.
