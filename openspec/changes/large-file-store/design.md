## Context

The current Rust rewrite has two table directions:

- The live TUI path reads input with `read_rows()` and builds `TableView` from `Vec<Vec<String>>`.
- `src/table/mod.rs` contains `TableStore`, `InMemoryTable`, and a prototype `LazyFileTable`.

This change should converge those into a single store-backed view architecture without regressing small-file simplicity.

## Decision

Use a store-backed table model instead of teaching the current `Vec<Vec<String>>` view to pretend it is lazy.

Small files should still use an in-memory store. Large seekable files should use an incremental file-backed store selected at or above `DEFAULT_LAZY_THRESHOLD_BYTES`. Stdin and non-seekable inputs should continue to materialize because they cannot support arbitrary row seeking without spooling.

## Architecture

Introduce a row access boundary that supports viewport rendering without requiring a full table:

```rust
trait TableStore {
    fn row_count(&self) -> RowCount;
    fn column_count(&self) -> usize;
    fn row(&mut self, index: RowIndex) -> Result<Option<RowRef<'_>>>;
    fn ensure_indexed_through(&mut self, index: RowIndex) -> Result<IndexProgress>;
    fn materialize(&mut self) -> Result<Vec<Vec<String>>>;
}
```

Use stronger wrapper types for row and column indices where they cross store/view boundaries:

```rust
struct RowIndex(usize);
struct ColumnIndex(usize);
enum RowCount {
    Exact(usize),
    AtLeast(usize),
    Unknown,
}
```

`TableView` should own cursor, viewport, header state, column metadata, and width state. It should query rows through the store boundary for rendering, movement, search, and operations.

## Initial Load

For large seekable files:

1. Open the file and read a bounded sample.
2. Decode enough bytes to choose encoding or fail early.
3. Sniff delimiter from sampled decoded rows.
4. Parse enough rows to classify header, compute initial column metadata, compute initial widths, and fill the first viewport.
5. Render the first frame.
6. Continue indexing row offsets lazily as navigation requires additional rows.

The default threshold remains:

```rust
DEFAULT_LAZY_THRESHOLD_BYTES = 100 * 1024 * 1024
```

The threshold remains centralized so later work can expose it as configuration.

## Incremental Indexing

The large-file store should keep row offsets and parser state sufficient to seek to already indexed rows and parse a requested row. Navigation beyond indexed data should call `ensure_indexed_through(target_row)`.

The implementation may index synchronously in bounded chunks at first. If UI pauses are noticeable, a later change may move indexing to a worker thread. This change does not require adopting an async runtime.

## Operations

Operations fall into three groups:

- Viewport-local operations: render, cursor movement inside indexed range, current cell popup, yank current cell.
- Incremental operations: search and skip-to-change may index progressively until they find a match or reach EOF.
- Full-table operations: sort requires materialization or a full row index. It must preserve state on failure and display a non-fatal status/progress path.

Numeric column profiling should use bounded sampling at initial load and keep the resulting profile sticky until reload/reclassification.

## Status and Errors

Long-running indexing or materialization should update a status message rather than silently freezing with no explanation. Errors should be non-fatal when possible and must leave the prior table state intact.

## Test Strategy

- Unit-test row index wrappers and store behavior.
- Test that large files can produce initial rows without full materialization.
- Test navigation that forces incremental indexing.
- Test search through non-indexed rows.
- Test sort materialization success and failure behavior.
- Add render tests for unknown or partially known row counts/status text.

## Risks

- CSV records can span multiple physical lines. The file-backed store must index logical CSV records, not naive newline offsets, unless a documented first implementation explicitly limits lazy mode to line-oriented dialects and falls back to materialization for complex quoted records.
- Encoding detection over bounded samples can differ from full-file detection. The store should preserve explicit `--encoding`, fail clearly on decode errors, and document accepted behavior.
- Full-table operations can still be expensive. They need clear status and state preservation.
