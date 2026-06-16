## Context

Tabview currently ships as a Python package with a `tabview` script and one primary curses implementation file. The supported user-facing behavior is the CLI/TUI experience documented in `README.rst`: load CSV-like tabular data, view it in a spreadsheet-like terminal interface, navigate with vim-like keys, search, sort, adjust column widths, reload, inspect cells, and yank cell contents.

The Rust rewrite is a hard replacement for the executable, not a translation of the Python internals. The Python import API is out of scope. The first Rust release targets `cargo install` on macOS, Linux, and WSL.

## Goals / Non-Goals

**Goals:**

- Preserve the `tabview` binary name and command-line compatibility.
- Preserve the current TUI layout, keybindings, and stateful interactions as closely as practical.
- Use Ratatui with the crossterm backend for terminal rendering and input.
- Build one Rust crate with one binary, organized around domain concepts rather than Python module structure.
- Support lazy/streaming access for very large files while retaining simple full-table operation for ordinary files.
- Use typestate where it prevents invalid input, parsing, and terminal lifecycle states.
- Add compatibility tests that compare selected Rust behavior with the existing Python implementation before replacement.

**Non-Goals:**

- No Python import API, PyO3 package, or Python compatibility wrapper.
- No spreadsheet editing, formulas, filtering, projections, richer data transforms, or Visidata-style feature growth.
- No package distribution beyond `cargo install` for the first Rust version.
- No attempt to preserve Python code structure, exception names, object layout, or internal quirks that are clearly bugs.

## Decisions

### One crate, explicit internal modules

Use a single Cargo package that builds one `tabview` binary. Organize internals by responsibility:

```text
src/
  main.rs             CLI entrypoint
  cli.rs              argument parsing and compatibility aliases
  ingest/             sources, encoding, CSV dialect, row normalization
  table/              table abstractions, row store, lazy access
  view/               cursor, viewport, column sizing, header state
  command/            key registry, modifiers, actions
  ops/                search, sort, skip-to-change, clipboard
  ui/                 Ratatui rendering, popups, help, search prompt
  compat/             compatibility fixtures and Python comparison helpers
```

Rationale: this keeps the public artifact simple while making the core behavior testable outside the terminal.

### Typestate only at meaningful boundaries

Use typestate for phase transitions where invalid combinations are otherwise easy:

```text
InputSource<Unread>
      |
      v
DecodedInput<EncodingKnown>
      |
      v
ParsedRows<Rectangular>
      |
      v
TableModel<HeaderClassified>
      |
      v
App<Running>
```

Use ordinary enums and structs for frequent runtime state such as current mode, active popup, column width mode, sort direction, and search direction. Rationale: typestate should prevent invalid construction, not make normal UI transitions hard to read.

### Ratatui and crossterm

Use Ratatui for rendering and crossterm for terminal backend/input. Keep rendering as a pure projection of app state where possible:

```text
crossterm events -> CommandRegistry -> AppState mutation -> Ratatui frame
```

Rationale: Ratatui gives stable layout primitives and testable buffer rendering, while crossterm supports the macOS/Linux/WSL target set without binding the design to curses.

### Data access model

Represent table access behind a trait that supports both in-memory and lazy stores:

```rust
trait TableStore {
    fn row_count(&self) -> Option<usize>;
    fn column_count(&self) -> usize;
    fn row(&mut self, index: usize) -> Result<RowRef<'_>>;
    fn reload(&mut self) -> Result<ReloadOutcome>;
}
```

Start with an in-memory store for compatibility tests and ordinary files, then add a lazy file-backed store that indexes row offsets incrementally. Operations that inherently require the full dataset, such as global sort, may materialize or build an index with clear progress/error handling.

Use a named configurable constant for the default lazy/indexed threshold:

```rust
const DEFAULT_LAZY_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;
```

Files at or above this 100 MiB threshold should use the lazy/indexed path by default. Keep the threshold centralized so later releases can expose it as a CLI/config option without changing ingestion architecture.

### CSV sniffing strategy

Python uses `csv.Sniffer`, which has no exact standard equivalent. The implementation should choose one of these before coding:

| Option | Description | Pros | Cons |
| --- | --- | --- | --- |
| A | Use the Rust `csv-sniffer` crate or similar maintained crate | Fastest path, less custom code | May diverge from Python edge cases |
| B | Implement a small compatibility-focused delimiter heuristic for common delimiters and space-delimited data | Predictable, easy to test | Less complete than Python Sniffer |
| C | Invoke a Python compatibility helper only in tests, not runtime, and tune Rust heuristic against fixtures | No runtime Python dependency, measurable compatibility | Requires a fixture matrix and explicit accepted divergences |

Decision: use Option C with a custom, fixture-tuned Rust heuristic and the standard `csv` crate for actual parsing.

The runtime sniffer should be intentionally small: honor explicit `--delimiter` first; otherwise sample decoded input, score common delimiter candidates such as comma, tab, semicolon, pipe, and space by quote-aware consistency of field counts, then apply Tabview's existing space-delimited normalization rule when space wins. Feed the selected delimiter, quote character, quoting mode, and flexible-row behavior into `csv::ReaderBuilder`.

Do not use `csv-sniffer` as the core runtime architecture. It infers more metadata than Tabview needs, such as header and type information, its reader path is less natural for stdin and lazy/streaming sources because it expects seekable readers, and it still would not match Python `csv.Sniffer` exactly. It may be used in a spike or compatibility comparison, but accepted behavior must be defined by Tabview fixtures and documented compatibility decisions.

### Dynamic key registry and help

Define keybindings once in a command registry. Render the help popup from that registry, grouped by command category. This preserves the current F1/`?` help behavior and prevents drift when key remapping or OS-aware modifier names are added later.

### Clipboard as optional feature

Provide a `clipboard` Cargo feature backed by `arboard`. When enabled, use `arboard` to copy the current cell text to the system clipboard. When disabled, when clipboard initialization fails, or when setting text fails, `y` must be a no-op or display a non-fatal status message; it must not crash the viewer.

Rationale: `arboard` provides a small OS-independent clipboard API, supports the macOS and Linux targets needed for the first Rust release, has Linux selection support for future refinement, and is focused enough for Tabview's current "copy this cell" behavior. Terminal-only or SSH-specific mechanisms such as OSC52 are out of scope for the first rewrite and can be considered later.

### Compatibility harness

Keep the Python implementation available long enough for tests to compare:

- CLI argument parsing fixtures.
- Encoding detection fixtures from `sample/`.
- CSV parsing and row padding fixtures.
- Header classification fixtures.
- Navigation/key sequence fixtures over small tables.
- Search and sort behavior fixtures.
- Snapshot-like Ratatui buffer tests for stable layout.

The harness should classify differences as either `expected-compatible`, `accepted-change`, or `bugfix`, so bug fixes do not get hidden as regressions.

### Compatibility classifications

Preserve these current behaviors:

- Space-delimited normalization with first-line `#`/`%` stripping.
- Row padding to a rectangular table.
- One-based user-visible cursor and start-position behavior with internal clamping.
- `file://` URI path extraction for local file paths.
- Header detection rule: a multi-row table treats the first row as a header when no first-row cell parses as numeric.
- Forward and reverse search cursor results, including wraparound behavior.

Treat these Python quirks as accepted bug fixes:

- Clipboard support should work on macOS without requiring `DISPLAY`; `arboard` replaces the current environment-gated subprocess fallback behavior.
- Empty cell popups should be a no-op instead of opening a blank popup.
- CSV sniffing should sample multiple decoded rows instead of only the first row.
- Header toggling should track header state structurally instead of deleting by `data.index(header)`, which can remove the wrong row when a data row equals the header row.
- Reverse search should not mutate the table while computing traversal; Rust should use reversed iterators or equivalent traversal over immutable table data.

Treat these changes as intentional enhancements:

- Encoding detection should try more specific encodings before permissive single-byte fallbacks. Latin-1 remains supported but should be a late fallback because it can decode nearly any byte stream.
- The default column width mode should be `mode`, aligning startup behavior with the documented intent and improving high-column-count files, instead of preserving the Python CLI's fixed-width `20` default.

## Risks / Trade-offs

- CSV sniffing may not match Python exactly -> build fixtures first, document accepted divergences, and keep delimiter overrides fully compatible.
- Lazy loading conflicts with global operations such as sort/search -> expose table-store capabilities internally and materialize/index when required.
- Ratatui layout can differ subtly from curses -> use buffer snapshots and manual smoke tests against screenshots/keybinding expectations.
- Typestate can overcomplicate UI code -> restrict it to construction/lifecycle boundaries and use plain enums for runtime modes.
- Removing the Python API may surprise existing users -> call this out in README/release notes and keep CLI compatibility as the supported migration path.
- Clipboard crates can introduce platform-specific failures -> make clipboard optional and non-fatal.

## Migration Plan

1. Add the Rust crate and compatibility tests while keeping the Python implementation available as a reference.
2. Implement and validate CLI parsing, ingestion, table model, and core operations outside the terminal.
3. Implement Ratatui UI and interactive command loop.
4. Run compatibility tests and manual TUI smoke tests on macOS, Linux, and WSL where available.
5. Replace the Python entrypoint/package metadata with Cargo-focused installation docs.
6. Remove Python runtime code from the supported implementation after the Rust binary reaches agreed parity.

Rollback during development is the existing Python implementation. After release, rollback is reinstalling the previous Python package or previous Rust crate version.
