## Why

Tabview can open delimited and structured text through `SourceAdapter`, `OpenedSource`, and `OpenedTable`, with stable source identities, typed cells, incremental access through `TableStore`, and source-neutral interactive or batch output. Users must still export SQLite tables before inspecting them, even though these contracts can support a query-native relational source directly.

Local SQLite support can therefore be added as a Turso-backed adapter and store without introducing a second table model or leaking database behavior into `TableView`.

## What Changes

- Add local SQLite as an explicit and signature-detected input format, opened through the `turso` crate.
- Discover user-facing ordinary SQLite tables and compatible ordinary views, exclude virtual, shadow, and internal objects, and select one with `--table`, saved `source.table`, automatic selection when exactly one selectable candidate exists, or a simple interactive table picker when multiple selectable choices remain. Preserve unavailable-view diagnostics so the picker or an explicit selection can explain incompatibility.
- Extend the existing source-identity model with relation-column identity; SQLite column metadata is supplied directly and is never represented as a synthetic CSV header row.
- Map Turso values directly into the existing typed `CellValue` variants. Preserve raw SQLite declarations for inspection, use SQLite affinity only as an initial logical-type hint, and widen the observed profile when runtime values disagree.
- Split row operations into a bounded source pipeline (`SourceFilter`, `SourceSort`, then `limit`) and a universal local view pipeline (`ViewFilter`, `ViewSort`, then search and rendering).
- Add separate Source and View configuration modals: Source owns adapter-dependent filters, sorting, limits, capabilities, and query provenance; View owns source-neutral local filters, sorting, and presentation. Keep Column Info as a contextual current-column editor whose filter and sort controls update View configuration, and keep existing quick filter and sort commands view-scoped.
- Default SQLite source results to 1,000 rows, make the limit configurable under saved-view `source`, never expand it automatically after local filtering, and distinguish source-result extent from visible row count.
- Execute SQLite source filters and sorts as application-generated parameterized SQL, retain the generated final SQL as query provenance, and expose it for copying or reuse.
- Refactor saved-view YAML into human-readable top-level `source` and `view` sections, with only `name` and `filenames` remaining at the document root.
- Preserve the existing format-neutral object interpretation option by moving `object_mode` into the nested `source` section with the other source-opening options.
- Implement `TursoTableStore` with asynchronous query replacement, bounded fetching, cached rows, stable identity where available, and local materialization bounded by the source result rather than the full database.
- Make SQLite available through the existing source-neutral output adapters. Direct batch output auto-selects a sole selectable relation but fails clearly when multiple selectable relations remain unresolved; interactive and interactive-export modes may use the table picker.
- Enforce the read-only user contract inside Tabview: keep the Turso connection behind a read-only application facade, expose only schema inspection and generated `SELECT` operations, enable and verify `PRAGMA query_only=ON` as defense in depth, and regression-test that every supported user action leaves logical database contents unchanged.
- Keep Turso's default features intentionally, including FTS and mimalloc, without treating Turso's FTS feature as compatibility with existing SQLite FTS virtual tables.

## Capabilities

### New Capabilities

- `sqlite-data-source`: Local SQLite detection, Turso connection behavior, relation discovery and selection, bounded SQL source queries, query provenance, Tabview-enforced read-only behavior, typed rows, incremental storage, and reload/error behavior.

### Modified Capabilities

- `table-source-model`: Represent selected multi-relation sources, stable relational identities, bounded source-query results, result extent, and query provenance through the shared table/store contract.
- `table-operations`: Separate source filters/sorts from universal view filters/sorts and keep search within the final local view.
- `cli-compatibility`: Add `sqlite` format selection and `--table <name>` with source-specific validation.
- `saved-views`: Replace the flat saved-view document with top-level `source` and `view` sections, persist both operation layers, and resolve relational columns through stable source identity.
- `non-interactive-output`: Render the complete bounded SQLite source result through existing output adapters and define non-interactive multi-table selection behavior.

## Impact

- Adds `turso` and the minimal Tokio support needed to drive its asynchronous local API. Turso's default FTS and mimalloc features remain enabled, but SQLite virtual tables remain outside the selectable relation set.
- Extends `InputFormat`, `OpenOptions`, `SourceOptionOverrides`, format resolution, `OpenedSource` relation selection, and source-neutral batch preparation.
- Refactors the flat `SavedView` schema and the single `TableQuery`/filter/sort state into explicit source-query and view-transform models; backward compatibility for the pre-change YAML shape is not required.
- Adds a SQLite `SourceAdapter`, SQL query compiler, asynchronous query coordinator, and `TursoTableStore`; the existing `TableDefinition`, typed cells, and store boundaries remain authoritative.
- Affects CLI and saved-view schemas, direct and post-interactive output preparation, reload behavior, compatibility fixtures, release builds, and user documentation.
- Does not add Turso Cloud URLs, arbitrary SQL, database editing, or relation management beyond the initial selection modal.
