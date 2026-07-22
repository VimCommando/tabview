## 1. Source and Table Model Foundations

- [x] 1.1 Introduce `InputFormat`, merged source-open options, distinct lazy/schema-scan thresholds, and validated RFC 6901 JSON Pointer values.
- [x] 1.2 Introduce typed `CellValue`, typed `Row`, raw-to-display conversion, and tests distinguishing null, empty text, booleans, integers, floats, text, binary, and structured JSON.
- [x] 1.3 Introduce stable row/column index wrappers, opaque generation-scoped `RowId`/`ColumnId`, `SourceGeneration`, `ColumnSourceIdentity`, `ColumnDefinition`, logical/type-origin metadata, `SchemaState`, and append-only `SchemaDelta` types.
- [x] 1.4 Introduce `TableDefinition`, `OpenedTable`, `OpenedSource`, relation metadata, and source-adapter/format-resolver interfaces with one implicit relation for current formats.
- [x] 1.5 Add automatic format probing plus explicit-format precedence tests for delimited, JSON, NDJSON, ambiguous content, and stdin.
- [x] 1.6 Introduce source-neutral `TableQuery`, `FilterSpec`, `SortSpec`, and `NullPlacement` types keyed by generation-scoped `ColumnId`, retaining explicit raw/rendered predicate domains, per-key resolved null placement, and current multi-filter/multi-sort behavior.

## 2. CLI and Pre-Open Saved View Resolution

- [x] 2.1 Add `--format auto|delimited|json|ndjson`, `--json-path`, and `--schema-scan default|full` parsing, help text, and validation tests.
- [x] 2.2 Preserve delimited option compatibility by making explicit CSV-only options imply delimited format under `auto` and rejecting incompatible explicitly selected formats.
- [x] 2.3 Extend saved-view parsing and semantic validation with top-level `format`, `json_path`, and `schema_scan` source options plus top-level and per-column `nulls: first|last` operation policy.
- [x] 2.4 Refactor saved-view discovery/selection so filename matching and source options are available before a table is opened.
- [x] 2.5 Merge source options with `explicit CLI > selected saved view > defaults` precedence and add override tests including `--schema-scan default` over a saved full scan.
- [x] 2.6 Extend `schemas/view.schema.json` and saved-view schema tests for source options, per-column `label`, and view/per-column `nulls` enums.

## 3. Delimited Adapter Compatibility

- [x] 3.1 Implement the delimited source adapter using existing decoding, delimiter sniffing, quoting, space normalization, and rectangular-row behavior.
- [x] 3.2 Move compatible first-record header classification from `TableView` into delimited table-definition construction.
- [x] 3.3 Construct stable delimited column definitions for named, duplicate, blank, and generated headerless columns without using display names as identity.
- [x] 3.4 Convert delimited cells to typed table rows as text while preserving empty-string behavior and existing numeric/view profiling semantics.
- [x] 3.5 Route ordinary delimited files and stdin through an `OpenedTable` backed by `InMemoryTable` and pass all existing ingestion/header fixtures.

## 4. Incremental Store Model

- [x] 4.1 Expand `TableStore` with partial `RowCount`, generation-scoped row access, `ensure_indexed_through`, bounded scan/fold traversal, byte/index progress, schema deltas, and controlled materialization.
- [x] 4.2 Update `InMemoryTable` to the expanded typed-row contract and add tests for exact counts, indexed access, no-op indexing, and materialization.
- [x] 4.3 Replace eager `LazyFileTable` indexing with incremental delimited logical-record indexing selected at `DEFAULT_LAZY_THRESHOLD_BYTES`.
- [x] 4.4 Preserve parser-provided offsets for quoted multi-line records and add seek/read tests for logical records crossing chunks.
- [x] 4.5 Define and test materialized fallback behavior for stdin, non-seekable sources, and encodings that cannot safely use byte offsets.
- [x] 4.6 Add failure tests proving indexing/materialization errors retain the last valid store state and progress metadata.
- [x] 4.7 Add an optional complete-query execution capability that distinguishes `Unsupported` from execution failure and returns a possibly incremental, generation-bound derived result store without mutating base source order.
- [x] 4.8 Detect observable seekable-source replacement, truncation, or incompatible mutation during incremental access and fail without mixing source generations or activating partial results.

## 5. Store-Backed View and Operations

- [x] 5.1 Rework `TableView` construction to own an opened table, derive headers/column profiles from `TableDefinition`, and remove direct header-row classification.
- [x] 5.2 Update rendering, cursor clamping, location/info text, and viewport sizing for `Exact`, `AtLeast`, and `Unknown` row counts.
- [x] 5.3 Centralize application of schema deltas so every per-column view vector/set is extended safely while existing IDs, labels, order, cursor, and viewport remain stable.
- [x] 5.4 Replace full-table clones used by current-cell popup, table info, rendered/raw yank, and viewport rendering with indexed row/cell access.
- [x] 5.5 Make navigation beyond the indexed range request bounded indexing and report non-fatal errors/status without corrupting cursor state.
- [x] 5.6 Replace materialized search helpers with progressive forward/reverse bounded store scans outside `TableQuery`, preserving wraparound semantics as indexing reaches the selected table end.
- [x] 5.7 Replace materialized skip-to-change helpers with progressive row scans outside `TableQuery` for forward/reverse row and column changes.
- [x] 5.8 Implement the canonical generic local query executor with stable base-order ties, direction-independent resolved first/last null ordering, typed numeric and existing textual fallback rules, canonical text/natural/regex behavior, explicit raw/rendered domains, and every existing comparison mode.
- [x] 5.9 Validate generation, column IDs, predicates, value domains, and modes before execution; then coordinate exact complete-query store execution, local fallback on `Unsupported`, separate execution failures, and atomic activation while allowing valid internal indexing/cache progress.
- [x] 5.10 Migrate sort application/clearing to stable-ID `TableQuery` state, resolve each key's policy with `column nulls > view nulls > last`, and preserve active multi-sort semantics and deterministic base source order when cleared.
- [x] 5.11 Migrate filter application/clearing to stable-ID `TableQuery` state, using complete local scans while accepting incremental exact source-result stores with partial counts.
- [x] 5.12 Preserve selected `RowId`/`ColumnId` and marks across successful query transitions, clamp when selection is filtered out, retain hidden marks within a generation, and invalidate row identities on reload.
- [x] 5.13 Implement shared sampled/exact scan-fold reductions for widths, numeric/type profiles, gradients, and identifier metadata without routing them through `TableQuery` or cloning the row table solely for aggregation.
- [x] 5.14 Define sampled initial width/profile behavior and controlled exact behavior for max widths, auto gradients, identifier metadata, and other full-dataset profiles.
- [x] 5.15 Enter the terminal before initial source opening, render `Loading <filename>` in the status bar until the first successful table-data frame atomically replaces it, restore terminal state on opening failure, and add later-operation status messages for schema scanning, substantial indexing, and materialization.
- [x] 5.16 Rework reload to create a new source generation, reopen through format resolution, discard stale row/query-result identities, and reapply cursor position where possible, viewport, widths, search, query configuration, and column settings by stable source identity.

## 6. JSON and NDJSON Parsing

- [x] 6.1 Add `serde_json` with raw-value support and implement reusable `StreamDeserializer`, `Visitor`, and `DeserializeSeed` helpers for streaming top-level documents, traversing selected JSON Pointer segments, skipping unrelated values, and retaining structured `RawValue` cells.
- [x] 6.2 Implement JSON/NDJSON extension and bounded-content detection without overriding explicit or saved-view format choices.
- [x] 6.3 Implement RFC 6901 pointer parsing/resolution, including escaped segments, missing paths, scalar selections, and the Elasticsearch `/hits/hits` fixture.
- [x] 6.4 Implement regular JSON row iteration for selected arrays and selected single objects while ignoring surrounding metadata outside the selected table.
- [x] 6.5 Implement NDJSON logical-document iteration and per-document starting-path resolution without naive newline splitting inside invalid/incomplete records.
- [x] 6.6 Implement recursive object flattening to row-relative canonical pointers, positional array-row columns, and atomic nested-array JSON cells.
- [x] 6.7 Preserve native JSON scalar types and implement monotonic inferred-type widening across null, integer, float, boolean, text, structured, and mixed values.

## 7. JSON Schema Discovery and Stores

- [x] 7.1 Implement first-seen canonical path collection and initial shortest-unique-suffix labels with unambiguous escaping/bracket notation for path-like keys.
- [x] 7.2 Implement `DEFAULT_SCHEMA_SCAN_BYTES` discovery over selected logical rows, finishing the row crossing 100 MiB and returning provisional or complete schema state.
- [x] 7.3 Implement `SchemaScan::Full` so every selected logical row contributes schema/type information without requiring all decoded rows to remain in memory.
- [x] 7.4 Implement append-only late columns, null padding for earlier rows, frozen existing labels/order, shortest non-conflicting late labels, and schema-delta tests.
- [x] 7.5 Implement incremental NDJSON row-offset indexing from `StreamDeserializer::byte_offset` and random indexed row decoding for large seekable inputs.
- [x] 7.6 Implement a shared counting reader around the `serde_json` selected-array visitor, record logical element boundaries around `next_element_seed`, normalize separator lookahead when needed, and prove indexed seek/reparse across whitespace, escaped strings, nested structures, chunk boundaries, and trailing surrounding metadata.
- [x] 7.7 Add materialized JSON/NDJSON stores for small and non-seekable inputs using the same table-definition and schema-discovery behavior.

## 8. Structured Columns and Saved Views

- [x] 8.1 Resolve structured columns by exact case-sensitive canonical pointer before allowing unambiguous display-label fallback; retain existing delimited exact/wildcard compatibility.
- [x] 8.2 Retain unmatched canonical column configuration while schema state is provisional, apply it when a late column arrives, and warn only after complete schema proves it missing.
- [x] 8.3 Apply and serialize per-column `label` overrides without changing source identity, raw values, or canonical sort/filter references.
- [x] 8.4 Apply, inherit, validate, and serialize view/per-column null placement; expose inherited/first/last in column information and atomically rerun an active sort when its effective policy changes.
- [x] 8.5 Show canonical structured source identity and source type in column information while rendering compact or overridden labels in headers.
- [x] 8.6 Reapply structured column configuration, sort, filters, and effective null placement correctly after reload and after append-only schema deltas.

## 9. Verification and Documentation

- [x] 9.1 Add JSON fixtures for top-level objects, arrays of objects, arrays of arrays, nested objects, nested arrays, mixed types, null versus empty text, path-like keys, and malformed input.
- [x] 9.2 Add Elasticsearch response fixtures and CLI/saved-view integration tests proving `/hits/hits` ignores metadata and exposes row-relative `_source` columns.
- [x] 9.3 Add generated inputs proving default 100 MiB schema discovery remains bounded, full scan reaches EOF, and columns appearing after the bound append without renaming existing headers.
- [x] 9.4 Add large delimited, JSON, and NDJSON integration tests proving initial open does not materialize every row, navigation indexes additional rows, and detected mid-generation source changes fail without mixing data.
- [x] 9.5 Add operation tests for progressive search/skip, default/view/column/per-key null placement in both directions, textual null placeholders, typed comparator and predicate semantics, stable base-order ties, row-identity cursor/mark behavior, query validation, incremental source-executed results, unsupported-to-local fallback, exact semantic equivalence, source-order restoration, scan/fold reductions, generation invalidation, and failure-state preservation.
- [x] 9.6 Add Ratatui render tests for the `Loading <filename>` first-frame lifecycle, source-defined headers, compact/overridden JSON labels, provisional schemas, late columns, partial row counts, and progress/error messages.
- [x] 9.7 Run formatting, linting, default-feature tests, no-default-feature tests, and release build verification.
- [x] 9.8 Update README and examples for supported formats, automatic/explicit selection, JSON Pointer starting paths, schema scan trade-offs, late columns, canonical saved-view keys, labels, view/per-column null placement, and large-file full-operation costs.
- [x] 9.9 Make default sampled widths fit the widest initially observed rendered value across source formats, freeze existing automatic widths after the initial sample, and add JSON/delimited regressions.
- [x] 9.10 Include the final partially visible column in viewport layout and clip its header/data cells at the terminal edge.
- [x] 9.11 Scale single-column `,`/`.` resizing by 20 percent per step, with one-character minimums and a widest-cached-rendered-value cap.
- [x] 9.12 Fill the first rendered terminal viewport by indexing through its calculated row capacity before drawing table data.
- [x] 9.13 Cap automatic column widths at 80 percent of the terminal viewport while allowing explicit widths and manual growth to exceed the cap.
- [x] 9.14 Make `G` index and load remaining rows with sequential batch scans without per-row file reopen/reparse or repeated sampled width/profile inference.
- [x] 9.15 Fuse delimited forward indexing and decoded-row delivery into one transactional parser pass, retain delivered rows in the active view cache, and benchmark the reported 115k-row CSV.
- [x] 9.16 Make rendered/raw current-cell yank pass only the selected cell to the clipboard layer without cloning visible rows.
- [x] 9.17 Route store-backed sort/filter replacement through the exact `QueryExecution` capability, retain lazy derived result stores, and restore base rows without reopening the source.
- [x] 9.18 Make query configuration and result activation atomic on source execution, materialization, validation, or local execution failure, with injected-store regressions.
- [x] 9.19 Render a schema-scan-specific opening status before an explicit full structured schema scan begins and cover the lifecycle message.
