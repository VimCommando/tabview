## 1. Store Model

- [ ] 1.1 Introduce row index and row count wrapper types for store/view boundaries.
- [ ] 1.2 Expand `TableStore` to support partial row counts, row access by index, incremental indexing, and materialization.
- [ ] 1.3 Keep `InMemoryTable` as the default store for ordinary files and stdin.
- [ ] 1.4 Rework `LazyFileTable` into an incremental store that can return initial rows without full materialization.

## 2. Ingestion and Store Selection

- [ ] 2.1 Add store selection based on `DEFAULT_LAZY_THRESHOLD_BYTES` for seekable filesystem inputs.
- [ ] 2.2 Keep stdin and non-seekable inputs on the materialized in-memory path.
- [ ] 2.3 Implement bounded initial sampling for encoding detection, delimiter sniffing, header classification, column metadata, widths, and first viewport rows.
- [ ] 2.4 Handle CSV logical records correctly when indexing row offsets, or explicitly fall back to materialization for dialects that cannot be indexed safely.

## 3. View Integration

- [ ] 3.1 Rework `TableView` to access rows through a store-backed model instead of directly owning all rows.
- [ ] 3.2 Preserve cursor, viewport, header state, column metadata, width mode, gap, mark, and search behavior across the store migration.
- [ ] 3.3 Update rendering to handle unknown or partially known row counts.
- [ ] 3.4 Keep numeric column profiles sticky using bounded sampling until reload or reclassification.

## 4. Operations

- [ ] 4.1 Keep viewport-local operations, cell popups, info popups, and yanking working without full materialization.
- [ ] 4.2 Make navigation index additional rows on demand.
- [ ] 4.3 Make search and skip-to-change progressively index rows until a result or EOF.
- [ ] 4.4 Make sort materialize or fully index in a controlled operation that preserves prior state on failure.
- [ ] 4.5 Add status reporting for long-running indexing and materialization.

## 5. Tests and Documentation

- [ ] 5.1 Add unit tests for row wrappers, partial row counts, in-memory store behavior, and lazy store behavior.
- [ ] 5.2 Add integration tests proving large seekable files render initial rows without full materialization.
- [ ] 5.3 Add tests for navigation-driven indexing, incremental search, and sort materialization.
- [ ] 5.4 Add Ratatui render tests for large-file status and unknown or partial row counts.
- [ ] 5.5 Update README or internal documentation to describe large-file behavior and full-table operation trade-offs.
