## Why

The viewer currently treats every input as an eagerly decoded CSV-like file and asks `TableView` to infer a header and columns from a fully materialized `Vec<Vec<String>>`. That architecture prevents fast access to large files and cannot correctly represent formats such as JSON and NDJSON whose column names, types, and table root come from format-specific structure rather than a CSV header row.

## What Changes

- Introduce a format-aware table-source boundary that opens an input as a table definition plus a row store, keeping schema discovery out of `TableView` and leaving room for database-backed sources in a follow-on change.
- Preserve typed cell values and stable column identities separately from display formatting, inferred profiles, and saved-view overrides.
- Give rows opaque identity scoped to an opened source generation so derived results can preserve selection, marks, deterministic tie order, and stale-result safety.
- Route the live TUI through in-memory or incremental `TableStore` implementations, with bounded initial work, partial row counts, on-demand indexing, controlled full-table operations, and non-fatal progress/error reporting.
- Add automatic and explicit format selection for delimited, JSON, and NDJSON inputs while preserving existing CSV parsing options.
- Add JSON and NDJSON table construction from object fields, recursive object flattening, typed scalar values, atomic array values, canonical JSON Pointer column identities, and compact unique display labels.
- Add a JSON starting-path option in the CLI and saved views so an embedded array, such as `/hits/hits` in an Elasticsearch search response, can be selected as the table while surrounding metadata is ignored.
- Discover JSON columns from up to the first 100 MiB of the selected table payload by default, allow an opt-in full schema scan, mark bounded schemas provisional, and append late-discovered columns without renaming or reordering existing columns.
- Move saved-view source options ahead of table opening, support canonical source-path column matching, display-label overrides, and view/per-column null placement for sorting, and retain pending configuration for columns discovered later.
- Classify existing operations as viewport-local, progressive, or full-table so large inputs are not accidentally materialized by rendering, navigation, search, skip, cell popups, or clipboard actions.
- Represent sort and filter configuration as source-neutral table queries keyed by stable column identity, execute them through a capability-aware boundary that may return lazy results, and retain a canonical local fallback for stores that cannot preserve the complete viewer semantics.
- Keep progressive search/skip and scan/reduction operations outside the query contract so row membership/order, navigation, and aggregate profiling remain distinct responsibilities.
- Keep SQLite and other database access out of this change; they will use the new source/schema/store boundary in a follow-on change.

## Capabilities

### New Capabilities

- `table-source-model`: Format resolution, opened-source/table boundaries, source generations, stable row/column identity, table definitions, typed cells, schema completeness, and schema updates.
- `json-ingestion`: JSON/NDJSON selection, starting paths, row construction, schema scanning, nested object flattening, column naming, and late-column behavior.
- `large-file-store`: Store-backed rendering, incremental logical-row indexing, partial counts, lazy-aware operations, status reporting, and reload behavior.

### Modified Capabilities

- `data-ingestion`: Replace future large-file groundwork with the live format-aware store-selection path while retaining existing delimited-input compatibility.
- `table-operations`: Derive headers from source column definitions, preserve operation semantics across partial or incrementally indexed stores, and define the query/execution boundary for sort and filter operations.
- `saved-views`: Add source format, JSON starting path, schema scan policy, canonical JSON column matching, pending late-column configuration, and column display-label overrides.
- `cli-compatibility`: Add format selection, JSON starting path, and schema scan controls without removing existing arguments.

## Impact

- Affects `src/ingest`, `src/table`, `src/view`, `src/ui`, `src/lib.rs`, CLI parsing, saved-view parsing/resolution, documentation, schemas, fixtures, and render/integration tests.
- Adds `serde_json` for streaming deserialization, borrowed/raw structured values, and typed JSON value handling.
- Changes internal row and column representations and the order in which saved views are selected and applied, but preserves the existing user-facing CSV behavior and keybindings.
- Establishes the prerequisite architecture for a later read-only SQLite/database-source change without implementing database access here.
