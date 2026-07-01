## Why

Tabview is a compact, useful spreadsheet-like CLI viewer, but its current Python/curses implementation is tightly coupled, difficult to evolve, and reads full input into memory before use. A Rust rewrite can preserve the user-facing CLI and TUI behavior while creating a maintainable architecture with stronger state invariants and a path to very large file support.

## What Changes

- Replace the Python implementation with a single Rust crate that builds one `tabview` binary.
- Preserve the command-line interface, including existing flags, `+y:x` start-position syntax, stdin mode with `-`, Python-style CSV quoting names, and the current vim-like keybindings.
- Rebuild the TUI on Ratatui with the crossterm backend, keeping the original display layout and interaction model as closely as practical.
- Introduce idiomatic Rust domain boundaries for input decoding, CSV/table parsing, table storage, viewport state, command handling, rendering, modal dialogs, search, sort, reload, and clipboard integration.
- Use typestate where it prevents invalid states, especially in data ingestion phases and terminal lifecycle handling, without forcing typestate into simple state transitions.
- Add a compatibility test harness that can compare Rust behavior against the existing Python implementation before the hard replacement is completed.
- Add lazy/streaming data access for very large files while preserving current behavior for ordinary files and list-like parsed tables.
- Make clipboard support optional at compile time and use Rust clipboard integrations rather than directly mirroring Python subprocess probing.
- Render help dynamically from the keybinding registry so future key remapping or OS-aware modifiers cannot drift from help text.
- **BREAKING**: Remove the Python import API (`import tabview as t; t.view(...)`) from scope. Users install and run the Rust `tabview` binary via `cargo install`.
- Explicitly exclude spreadsheet editing, formulas, filtering, and other new viewer features not present in the current implementation.

## Capabilities

### New Capabilities

- `cli-compatibility`: Command-line interface compatibility for the replacement `tabview` binary.
- `data-ingestion`: File, stdin, encoding, delimiter, quoting, row normalization, and lazy/streaming table ingestion behavior.
- `tui-navigation`: Ratatui-based spreadsheet-like TUI layout, keybindings, viewport movement, modals, and reload state preservation.
- `table-operations`: Search, sorting, header handling, column sizing, skip-to-change, and cell clipboard operations.
- `rust-architecture`: Rust crate structure, typestate boundaries, testing strategy, packaging, and non-goals for the rewrite.

### Modified Capabilities

- None. This repo has no existing OpenSpec capability specs.

## Impact

- Affected code: replace `bin/tabview`, `tabview/tabview.py`, `tabview/__init__.py`, Python package metadata, and Python tests with a Rust crate and compatibility-focused test suite.
- Affected user API: preserve the `tabview` executable interface; remove Python import usage from the supported surface.
- New dependencies: Rust toolchain, Ratatui, crossterm, CSV parsing/encoding crates, Unicode width handling, argument parsing, optional clipboard crate(s), and test tooling.
- Packaging: first Rust release is installed with `cargo install`; Python package publishing is out of scope for this change.
- Compatibility risk: exact CSV sniffing behavior, edge-case curses rendering quirks, and large-file lazy loading require explicit design choices and regression tests.
