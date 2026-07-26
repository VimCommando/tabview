# Turso build and runtime impact

Tabview's default-enabled `sqlite` Cargo feature activates the optional
`turso` 0.7.1 dependency with default features disabled, explicitly enables
Turso's mimalloc integration as Tabview's global allocator, and retains
Tokio's multi-thread runtime and synchronization support for local
asynchronous I/O. A build without `sqlite` omits Turso, mimalloc, and Tokio
from its normal dependency graph. Tantivy-backed Turso FTS is not enabled.

The release measurement below was taken from an incremental build state on
macOS in July 2026 with `cargo build --release --all-features`. Exact values
vary with Rust version, target, linker, debug-symbol policy, and warmed build
cache, so they are an engineering reference rather than a release-size
promise.

| Measurement | Value |
| --- | --- |
| Platform | macOS 26.5.2, Apple Silicon (`aarch64-apple-darwin`) |
| Rust | 1.90.0, LLVM 20.1.8 |
| Turso | 0.7.1, defaults disabled; `mimalloc` enabled explicitly |
| Tokio features | `rt-multi-thread`, `sync` |
| Allocator | mimalloc through Turso's explicit `mimalloc` feature |
| FTS | Disabled; Tantivy absent from the normal graph and lockfile |
| Release binary | 19,665,440 bytes (18.8 MiB reported by `ls`) |
| Incremental release rebuild after feature change | 7.55 seconds wall clock |

Turso's optional FTS feature is disabled because Tabview does not expose its
functionality. Tabview also does not select existing virtual tables, including
FTS5 and RTree tables, because their behavior and module availability cannot
be treated like an ordinary table under the read-only source contract. Virtual
shadow tables and SQLite-internal objects are omitted from selection.

The test suite uses a bundled reference SQLite build only as a development
dependency to create genuine FTS5, RTree, shadow-table, and unavailable-module
fixtures. This is intentional: the pinned Turso build reports `no such module:
fts5` for SQLite's `fts5` module because Turso's optional `fts` feature is
disabled. The reference fixture does not ship in the release binary.

The runtime SQLite surface is intentionally narrow. A private facade opens the
database through Turso core with `OpenFlags::ReadOnly` before creating a
connection, verifies `PRAGMA query_only` as defense in depth, and exposes only
typed schema discovery, prepared query, and row-fetch operations. Tabview does
not expose arbitrary SQL execution. This storage-level boundary prevents
rollback-to-WAL conversion, sidecar creation, and writes to existing database
or sidecar bytes.

Native all-feature and no-default-feature builds and tests pass on the recorded
macOS platform. The all-feature test suite and all-target, all-feature Clippy
with warnings denied also pass on Fedora 44 x86_64 with Rust 1.95.0. Fedora's
optional `util-linux-script` package was represented by an isolated PTY shim
for the six integration tests that require the `script` command; no system
packages were installed. Tabview's other target environment is WSL, which is
the same supported Linux target family; native Windows and MinGW are not
compatibility targets for this change.
