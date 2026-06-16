## 1. Rust Project Scaffold

- [ ] 1.1 Create a single Cargo crate that builds a `tabview` binary.
- [ ] 1.2 Add initial dependencies for argument parsing, Ratatui, crossterm, CSV parsing, encoding, Unicode width, error handling, and testing.
- [ ] 1.3 Define module boundaries for `cli`, `ingest`, `table`, `view`, `command`, `ops`, `ui`, and `compat`.
- [ ] 1.4 Add `cargo fmt`, `cargo clippy`, and `cargo test` commands to the documented development workflow.

## 2. Compatibility Harness

- [ ] 2.1 Preserve access to the current Python implementation as a temporary compatibility oracle during the rewrite.
- [ ] 2.2 Add fixtures for CLI parsing, start-position parsing, encoding detection, CSV parsing, row padding, header classification, search, sort, popup behavior, clipboard behavior, and navigation key sequences.
- [ ] 2.3 Implement comparison tests that classify differences as compatible, accepted bug fix, intentional enhancement, or regression.
- [ ] 2.4 Add explicit accepted-change fixtures for macOS clipboard behavior, empty-cell popups, multi-row CSV sniffing, structural header toggling, non-mutating reverse search, reordered encoding detection, and default `mode` column width.
- [ ] 2.5 Add Ratatui buffer snapshot tests for the initial layout, header layout, selected cell rendering, and modal popups.

## 3. CLI Compatibility

- [ ] 3.1 Implement `tabview` argument parsing for filename, `-`, `--encoding`/`-e`, `--delimiter`/`-d`, `--quoting`, `--start_pos`/`-s`, `--width`/`-w`, `--double_width`, and `--quote-char`/`-q`, with default `--width mode`.
- [ ] 3.2 Implement classic `+y:x`, `+y:`, and related start-position parsing.
- [ ] 3.3 Map Python-style quoting names to Rust parser configuration.
- [ ] 3.4 Implement stdin loading while restoring terminal input for the interactive TUI.
- [ ] 3.5 Add CLI compatibility tests for README-documented invocations and MySQL pager usage.

## 4. Data Ingestion

- [ ] 4.1 Implement input source handling for paths, `file://` URIs, and stdin.
- [ ] 4.2 Implement encoding override and compatibility encoding detection with Latin-1 as a late fallback.
- [ ] 4.3 Select and implement the CSV sniffing strategy from the design options.
- [ ] 4.4 Implement explicit delimiter, quote character, and quoting mode behavior.
- [ ] 4.5 Implement space-delimited normalization with first-line `#`/`%` stripping.
- [ ] 4.6 Implement row padding so parsed data is rectangular.
- [ ] 4.7 Implement an in-memory table store for ordinary files and operation tests.
- [ ] 4.8 Implement lazy or streaming-backed table access for very large files.

## 5. View Model and Commands

- [ ] 5.1 Implement header classification and toggle behavior.
- [ ] 5.2 Implement cursor, viewport, visible-column calculation, and resize-aware layout state.
- [ ] 5.3 Implement fixed, mode, and max column width calculations with double-width character handling.
- [ ] 5.4 Implement key modifier accumulation and command dispatch.
- [ ] 5.5 Implement the full existing navigation keymap and mark/return-to-mark behavior.
- [ ] 5.6 Implement reload state capture and restoration.

## 6. Table Operations

- [ ] 6.1 Implement forward, reverse, and incremental search traversal.
- [ ] 6.2 Implement lexical, natural, and numeric sort operations for the current column.
- [ ] 6.3 Implement skip-to-change operations for row and column value changes.
- [ ] 6.4 Implement current cell yanking behind an optional clipboard feature.
- [ ] 6.5 Define controlled behavior for full-table operations against lazy table stores.

## 7. Ratatui Interface

- [ ] 7.1 Implement the main Ratatui/crossterm terminal lifecycle with restoration on normal exit and error.
- [ ] 7.2 Render the top location/status line, current cell contents, divider, optional header row, and visible table cells.
- [ ] 7.3 Render selected-cell highlighting and truncated cell content with Unicode-aware width handling.
- [ ] 7.4 Implement full cell, file/data info, and help popups.
- [ ] 7.5 Implement the incremental search prompt.
- [ ] 7.6 Generate help text dynamically from the command registry.

## 8. Replacement and Documentation

- [ ] 8.1 Update README usage and installation docs for `cargo install`.
- [ ] 8.2 Document that the Python import API is removed and out of scope.
- [ ] 8.3 Remove or retire Python packaging files from the supported implementation path.
- [ ] 8.4 Ensure no editing, formulas, filtering, or unrelated new features were introduced.
- [ ] 8.5 Run compatibility tests, Rust unit tests, render tests, and manual TUI smoke tests on macOS/Linux/WSL where available.
