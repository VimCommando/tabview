## Context

Tabview opens data through `SourceAdapter`, represents an opened source as `OpenedSource`/`OpenedTable`, describes its schema with `TableDefinition`, preserves raw values in typed `CellValue`s, and accesses rows through `TableStore`. The same table/view model now feeds both the TUI and source-neutral batch output adapters.

SQLite adds a query-native source that may contain millions of rows. Treating every sort or filter as a universal local operation would either materialize the database or make the user wait for unbounded work. At the same time, Tabview's regex, natural ordering, rendered-value matching, numeric suffix handling, and other presentation-aware operations should remain available independently of source capabilities.

The data path therefore needs two explicit stages: source operations decide which bounded working set is fetched, and view operations transform only that working set locally. The same distinction belongs in saved-view configuration and query provenance.

Turso's local Rust API is asynchronous and supports both reads and writes. Tabview needs a narrow application boundary that exposes only viewing operations and keeps potentially expensive source queries from blocking the terminal event loop.

This change covers local SQLite-format files. Turso Cloud and `libsql://` URLs remain outside scope.

## Goals / Non-Goals

**Goals:**

- Open a local SQLite database through Turso and view one selected ordinary table or compatible ordinary view.
- Guarantee that Tabview opens SQLite files at the storage boundary as read-only, cannot modify logical schema or data, does not convert rollback-journal databases to WAL, and does not create or modify engine sidecars.
- Bound every SQLite working set with a configurable source limit that defaults to 1,000 rows.
- Apply source-native filters and sorting before the source limit.
- Apply universal Tabview filters and sorting after the fixed source result is fetched.
- Never expand the source limit automatically when local view filtering hides rows.
- Preserve and expose the final generated SQLite source query for reuse.
- Keep expensive source-query replacement asynchronous and retain the last valid result until its replacement is ready.
- Represent source-result extent, visible rows, and stable row identity honestly.
- Organize saved-view YAML into top-level `source` and `view` sections.
- Present a simple table-selection modal in interactive execution only when multiple selectable user-facing relations remain unresolved.
- Support SQLite in direct batch output and post-interactive output without a source-specific exporter.
- Disable Turso's default feature set, retain mimalloc explicitly as Tabview's global allocator, and omit Tantivy-backed FTS because it is unrelated to the read-only table source.

**Non-Goals:**

- Turso Cloud, embedded replicas, sync, or remote URLs.
- Arbitrary user SQL or database editing.
- Automatically fetching more source rows to compensate for a selective view filter.
- Promising that `LIMIT` alone makes an unindexed database sort or scan fast.
- Making source-native and view-native filter or comparison semantics equivalent.
- Exact total-match counts for a limited source query.
- Exposing SQLite virtual tables in the initial implementation, including FTS, RTree, and extension-provided modules.

## Decisions

### Extend the existing adapter and store boundaries

Add `InputFormat::Sqlite` and a `SqliteAdapter` implementing `SourceAdapter`. The adapter performs bounded signature probing, relation discovery, source-option resolution, schema construction, and creation of a `TursoTableStore`.

The selected relation uses the existing table contract:

```text
OpenedTable
├── TableDefinition
│   ├── RelationMetadata
│   └── Vec<ColumnDefinition>
└── Box<dyn TableStore>
```

`OpenedSource` exposes the discovered relation catalog, an optional already-selected `OpenedTable`, and a source-owned way to open one catalog entry lazily. Existing single-relation adapters return their implicit table immediately. SQLite opens an explicitly selected or sole relation immediately; when multiple relations remain, it retains the source session until the application applies the selection policy for the resolved execution mode. It never creates a row stream for every relation.

### “Bypass CSV header” means metadata does not become a row

SQLite result metadata becomes `ColumnDefinition`s directly. The adapter never prepends a synthetic header row and never asks `TableView` to classify the first query row.

The delimited adapter continues to own its header heuristic: it may consume the first parsed record as column metadata or retain it as data. Both adapters produce the same table-definition contract after their source-specific decisions.

Runtime behavior targets `ColumnId`, durable remapping uses typed `ColumnSourceIdentity`, and rendering uses `display_name`. A generic reference-name field is not added.

### Add relational source and row identity

Extend `ColumnSourceIdentity` with a relational variant conceptually equivalent to:

```rust
RelationColumn {
    relation: String,
    ordinal: usize,
    name: String,
}
```

The relation plus ordinal distinguishes duplicate names. Saved views use a unique source name directly and deterministic one-based occurrence suffixes such as `name#2` for duplicates.

SQLite row identity uses a stable database key when one is available: rowid for ordinary rowid tables or the declared primary-key tuple for `WITHOUT ROWID` tables. Generated source SQL may select that key as hidden metadata and use it as a stable ordering tie-breaker. Ordinary views are treated as having no stable row identity in this implementation, even when their projected columns resemble a key. Views or other relations without a stable key reset cursor and mark state on source-query replacement.

### Resolve format before opening and select one relation lazily

Explicit `--format sqlite` or saved `source.format: sqlite` selects the adapter. In automatic mode, the strong `SQLite format 3\0` signature selects SQLite before text decoding. Stdin is not treated as SQLite.

`--table <name>` and saved `source.table` select a discovered ordinary table or compatible ordinary view. CLI values override saved values. Without selection, one selectable candidate is selected automatically and zero selectable candidates produce a clear error.

Selection of multiple candidates belongs to application orchestration rather than the adapter:

- Interactive and interactive-export execution displays a simple “Select table” modal before a row stream is opened when more than one selectable candidate remains. Confirmation opens only the selected relation; cancellation ends opening cleanly.
- Direct batch execution never waits for user input. It fails before writing stdout with a diagnostic requiring `--table` or saved `source.table`.

Resolved names are matched against discovery metadata before safely quoted SQL is constructed.

### Classify and capability-gate SQLite objects

Discovery classifies SQLite schema objects as ordinary tables, ordinary views, virtual tables, shadow tables, or internal objects rather than treating every schema entry as an equivalent table.

User ordinary tables are selectable. Each ordinary view must first pass a non-mutating queryability and result-metadata probe equivalent to preparing `SELECT * FROM <resolved-view> LIMIT 0` through the pinned Turso version. A successful probe makes the view selectable; it does not suppress a later runtime error if evaluating the view encounters a data-dependent incompatibility.

An ordinary view that fails the probe remains available as diagnostic catalog information but is not a selectable candidate. When the table picker is already required, it displays such a view disabled with the compatibility reason. Explicit `--table` or saved `source.table` selection of that view fails with the same actionable reason. When no selectable objects remain, startup distinguishes an incompatible database from an empty one.

Virtual tables are not selectable in the initial implementation, even when Turso happens to register the named module. A requested virtual table produces an unsupported-object diagnostic rather than a misleading missing-table error. Shadow tables and SQLite internal objects are excluded completely from the user-facing catalog.

Table metadata comes from SQLite schema metadata. View columns come from the prepared result metadata and use conservative logical-type hints because declared types and key information may be incomplete. Hidden virtual-table control columns are never exposed because virtual tables themselves are excluded.

Only selectable candidates participate in automatic-single and multiple-choice decisions. Disabled views therefore do not cause a modal to appear when exactly one selectable table or view remains.

### Split source queries from view transforms

The persistent operation model is divided conceptually into:

```rust
struct SourceQuery {
    generation: SourceGeneration,
    filters: Vec<SourceFilter>,
    order_by: Vec<SourceSort>,
    limit: NonZeroUsize,
}

struct ViewTransform {
    filters: Vec<ViewFilter>,
    order_by: Vec<ViewSort>,
}
```

The execution order is fixed:

```text
source rows
  → SourceFilter
  → SourceSort
  → source limit
  → ViewFilter
  → ViewSort
  → search and rendering
```

Changing a source filter, source sort, selected table, or limit replaces the source result. Changing a view filter or view sort never reopens or re-queries the source. Search traverses the rows remaining after view transforms and never becomes a persistent source clause.

The generic local executor remains authoritative for `ViewFilter` and `ViewSort`, including regex, raw/rendered matching, natural order, numeric suffixes, semantic versions, IP addresses, dates, booleans, custom null placement, and stable local ties. It materializes at most the already-limited source result.

`SourceFilter` and `SourceSort` deliberately use source-native semantics. SQLite supports a small typed vocabulary that maps directly to parameterized `WHERE` and `ORDER BY` clauses. File adapters may implement streaming source filters over decoded logical records; a quoted multi-line CSV record is one record rather than several physical lines. Source sorting is capability-dependent because sorting before a limit may require a complete source scan.

There is no automatic fallback from an unsupported source operation to materializing an unbounded source. The source reports the operation as unavailable with a reason. Users can express richer behavior in the view layer after fetching a bounded working set.

### Separate Source and View configuration modals

Runtime configuration is split by execution scope rather than by operation type:

```text
┌─ Source Configuration ──────────────────┐
│ Source/table identity and capabilities  │
│ Limit and result extent                 │
│ Source filters                          │
│ Source sort                             │
│ Query plan or generated SQL             │
│                         [Cancel] [Apply] │
└─────────────────────────────────────────┘

┌─ View Configuration ────────────────────┐
│ View filters                            │
│ View sort                               │
│ View-wide and column presentation       │
└─────────────────────────────────────────┘
```

The Source modal constructs the bounded working set and is intentionally adapter-dependent. Its common shell shows source identity, current limit and result extent, source filters, source sort, capability explanations, and query provenance when available. SQLite exposes typed native predicates, native sorting, and generated SQL. Delimited and structured-file adapters expose supported logical-record filters, omit SQL, and show source sorting as unavailable when they cannot produce it within their resource contract.

Source edits are staged in the modal. Confirming Apply validates the complete draft and starts at most one asynchronous source-query replacement; Cancel leaves the active query untouched. Pending, failure, and capability states remain inside the Source workflow, with the last successful result retained until replacement succeeds.

The View modal transforms and presents the already-bounded source result. Its filters, sort modes, null handling, column presentation, and interaction semantics are identical for delimited, structured, and SQLite sources. View edits may update the local result immediately because they do not re-query or expand the source.

A column may participate independently in both scopes. Source configuration determines whether its row enters the bounded working set; View configuration may then filter or reorder that same column locally.

Existing direct filter and sort commands—including the current-column filter prompt and shortcut sorts—remain View operations on every source. This preserves their existing responsiveness and prevents a familiar key from unexpectedly launching a potentially expensive database query. Source filters and sorts are changed only through the explicit Source modal.

Column Info remains a contextual editor for the current column. In addition to source identity, inferred or declared type, and presentation controls, it keeps direct View filter and View sort controls for that column. Changes made there update the same `ViewTransform` represented by the broader View Configuration modal; they are not a separate operation store. A current-column sort follows the established View sort precedence behavior, and a filter becomes a normal entry in `view.filters`.

The broader View modal provides the multi-column picture: ordered sort precedence, filters across columns, view-wide null behavior, and presentation configuration. Changes from either UI are reflected immediately in the other. Source operations affecting the current column may be summarized read-only in Column Info, but editing them requires Source Configuration so a contextual action cannot unexpectedly replace the source result.

The runtime View Configuration modal is distinct from the Saved View modal that displays or writes YAML. Runtime source and view state continue to serialize into the matching saved `source` and `view` sections.

### Make the source limit a hard working-set boundary

SQLite uses a default source limit of 1,000 when saved configuration does not specify `source.limit`. A positive configured value replaces that default.

SQLite may execute a private `LIMIT N + 1` probe, retain at most `N`, and use the extra row to distinguish a complete result from a truncated one. The reusable logical source query still has `LIMIT N`; the probe variant is execution bookkeeping and is not presented as the user's final SQL. The result contract exposes source extent separately from normal row access, conceptually:

```rust
enum ResultExtent {
    Complete { source_rows: usize },
    Truncated { source_rows: usize, limit: usize },
}
```

A view filter that reduces 1,000 fetched rows to 17 leaves 17 visible rows. Tabview does not fetch another 983. Status and information views distinguish visible rows from fetched source rows, for example `17 visible / 1,000 source rows (limited)`.

`goto-bottom`, exact reductions, local materialization, search, and view transforms stop at the fixed source-result boundary.

### Compile a small source plan rather than arbitrary SQL

The SQLite facade compiles the selected relation, typed source predicates, source sort keys, stable tie-breaker, and limit into one parameterized `SELECT`. Identifiers come only from resolved source metadata and values are bound parameters.

The source-filter vocabulary includes direct operations such as equality, inequality, ordered comparison, contains, prefix, and null tests. It does not attempt to reproduce view-filter semantics. Likewise, source sorting uses SQLite-native value and collation behavior rather than Tabview's natural or presentation-aware comparators.

This is a query renderer over a closed typed model, not a parser or validator for arbitrary user SQL.

### Preserve final source-query provenance

Every successfully compiled SQLite source result carries a query artifact containing the logical parameterized SQL, bound values, and a safely rendered copyable SQL statement. The TUI can display or copy that artifact for use elsewhere. Private execution changes used only for truncation detection do not alter the logical artifact.

The artifact represents only the source stage. When view filters or sorts are active, the SQL display identifies them as local transformations that are not represented by the source SQL.

Clearing a source operation generates a new artifact. View-only changes do not change the source artifact.

### Replace source results asynchronously and atomically

Turso connection setup, query execution, and row iteration run behind an application-owned Tokio boundary. A source-query change creates a revisioned query job:

1. Keep the previous successful source result and view state visible.
2. Display source-query progress.
3. Compile and begin the replacement query asynchronously.
4. Load a bounded initial viewport.
5. Atomically activate the replacement if its revision is still current.
6. Discard or cancel stale work when a newer source query supersedes it.

`TursoTableStore` owns one active result stream, a cached prefix of typed rows, source-result extent, schema deltas, identity metadata, and query provenance. Rendering never performs an asynchronous call per cell.

### Reuse source-neutral batch output

SQLite does not add an output formatter. After table selection, it produces the same `OpenedTable` and `TableView` consumed by the existing output driver.

For direct batch output, source opening and the initial source query complete before the adapter prepares output. The output adapter's request for complete rows drains only the bounded active source result, applies the configured `ViewTransform`, resolves widths, and then writes stdout. It never expands to the underlying database or refills rows hidden by a view filter.

For `--interactive --output <format>`, the selection modal is available during startup. On normal quit, final export awaits the latest requested source-query revision before freezing the view, so successfully requested source changes are reflected in output. Failure during that preparation follows the existing no-partial-output contract.

Batch diagnostics and query failures use stderr. Generated SQL is viewable or copyable through its dedicated UI action; it is not mixed into normal serialized table stdout.

### Preserve Turso values as typed cells

Turso values map directly into the existing model:

| Turso value | `CellValue` |
|---|---|
| `Null` | `Null` |
| integer | `Integer(i64)` |
| real | `Float(f64)` |
| text | `Text` |
| blob | `Binary(Vec<u8>)` |

No value is converted to display text at ingestion.

SQLite declared types are advisory metadata, not conversion rules. The adapter preserves the raw declared type for Column Info and derives only an initial `LogicalType` hint using SQLite's ordered affinity rules:

| SQLite declaration result | Initial `LogicalType` hint |
|---|---|
| INTEGER affinity | `Integer` |
| REAL affinity | `Float` |
| TEXT affinity | `Text` |
| Explicit `BLOB` declaration | `Binary` |
| NUMERIC affinity | `Unknown` |
| No declaration or `ANY` | `Unknown` |

`TypeOrigin::Declared` means that the initial hint came from schema metadata; it does not assert that every value has that type. Semantic-looking NUMERIC declarations such as `BOOLEAN`, `DATE`, `DATETIME`, and `DECIMAL` remain `Unknown`. Tabview does not infer Boolean, date, time, or exact-decimal behavior from the spelling alone.

Runtime Turso values remain authoritative. As values arrive, the normal column profile widens from the hint: compatible values preserve it, numeric integer/real combinations widen to `Float`, and contradictory storage classes widen to `Mixed`. `NULL` does not contradict a hint. The same rule applies to non-STRICT and STRICT tables; STRICT improves the database's enforcement but does not change Tabview's ingestion contract. A view expression without reliable declared metadata begins `Unknown` and is inferred only from returned values.

Binary display remains a source-neutral presentation concern.

### Refactor saved views into source and view sections

Only document identity and discovery metadata remain at the root:

```yaml
name: recent-transactions
filenames:
  - transactions.db

source:
  format: sqlite
  table: transactions
  limit: 1000
  filters:
    - column: status
      operator: equal
      value: settled
  sort:
    - column: timestamp
      direction: desc

view:
  locale: en_US
  nulls: last
  filters:
    - column: description
      action: in
      kind: regex
      condition: 'invoice-\d{6}'
  sort:
    - column: user
      direction: asc
      kind: natural
  columns:
    amount:
      type: number
      format: locale
      align: right
```

`source` contains format selection, table or structured starting path, format-neutral object interpretation, schema-scan policy, result limit, source filters, and source sorting. `view` contains locale, null policy, column presentation, view filters, and view sorting. SQLite ignores the default `object_mode: auto`; explicit incompatible object modes are rejected by normal source-option validation.

The pre-change flat saved-view shape is not retained. CLI source options merge into `source`, while interactive operations serialize back into the layer they modify.

### Enforce read-only behavior at the Tabview boundary

The raw Turso connection is private to a Tabview-owned facade. The facade exposes typed relation discovery, schema inspection, source-query compilation, and row-query operations; it exposes no general execute method and accepts no user SQL.

The facade bypasses the high-level `turso::Builder::new_local` path because Turso 0.7.1 opens that path with create/write flags and may convert a legacy rollback-journal database to WAL before a connection exists. Instead, it constructs a Turso core database with `OpenFlags::ReadOnly` before connecting. Existing WAL files remain readable, but a missing WAL or shared-coordination file is not created.

Immediately after connecting, the facade enables and verifies `PRAGMA query_only=ON` as defense in depth. If verification fails, opening fails. Regression tests snapshot database bytes and any existing WAL or shared-memory files, exercise supported reads and rejected mutations, and verify that no file content or sidecar set changed.

### Select Turso features explicitly

Add `turso` with default features disabled, then explicitly enable mimalloc as
Tabview's global allocator while leaving Tantivy-backed FTS out of the release
dependency graph. Existing SQLite FTS3/4/5 virtual tables remain unselectable.
Keep only the Tokio features required by the runtime boundary and record
binary-size, compile-time, allocator, and platform effects.

### Gate SQLite behind a default feature

Add a default-enabled `sqlite` Cargo feature that activates the optional
`turso` and `tokio` dependencies together with the SQLite adapter. The normal
build keeps its SQLite behavior unchanged.

A build without `sqlite` omits the adapter module and dependency graph,
`InputFormat::Sqlite`, SQLite signature dispatch, and the `--table` CLI option.
Its format diagnostics and help text list only the formats compiled into that
binary. The source/view table model remains source-neutral and available to
file adapters.

## Risks / Trade-offs

- **`LIMIT` does not bound database work for an unindexed scan or sort** → Run source queries asynchronously, keep the last valid result visible, support stale-job cancellation, and report progress.
- **Source and view operations can look similar while producing different result sets** → Label their scope explicitly in commands, modals, saved YAML, status, and query output.
- **View filtering may leave very few visible rows** → Report visible and source counts separately and never refill implicitly.
- **A source lacks a stable row key** → Reset identity-dependent cursor or mark state across source-query replacement instead of guessing.
- **A future code path could use a mutating Turso API** → Open Turso core with `OpenFlags::ReadOnly`, keep the raw connection private, verify `query_only`, and test physical and logical before/after snapshots.
- **A file SourceSort may require unbounded work** → Make source-operation capabilities explicit and retain bounded view sorting as the universal alternative.
- **Batch execution cannot ask which table to open** → Auto-select only a sole selectable candidate and require explicit table selection when multiple selectable candidates remain.
- **A stored view can use SQL Turso cannot execute** → Probe every ordinary view with the pinned Turso version, retain unavailable reasons, and preserve normal query-failure handling for data-dependent incompatibilities.
- **Virtual-table support depends on registered modules and module-specific semantics** → Exclude all virtual tables from the initial selectable catalog and test that virtual, shadow, and internal objects receive the intended diagnostics.
- **Duplicate database column names are legal** → Include ordinal in source identity and use deterministic suffixed saved-view keys.
- **Turso defaults add unrelated FTS code alongside the desired allocator** → Disable default features, enable only mimalloc, and verify Tantivy is absent from normal release builds.
- **Feature-disabled builds could advertise unavailable SQLite behavior** → Compile-gate the format variant, signature probe, table-selection CLI, adapter, tests, and user-facing format diagnostics together.

## Migration Plan

1. Refactor saved-view and runtime query state into explicit source and view sections.
2. Add bounded source-query, result-extent, provenance, and asynchronous replacement contracts.
3. Add SQLite format selection, relational identities, discovery, and the table picker.
4. Implement the storage-level read-only Turso core facade, typed SQLite adapter, source-query compiler, and limited incremental store.
5. Integrate view transforms, batch output, reload, identity behavior, SQL output, status, and errors.
6. Run physical and logical read-only, query-layer, boundedness, adapter, interactive, batch-output, and platform verification.

Rollback removes SQLite dispatch and dependencies. The generic source/view query split and nested saved-view structure remain useful for file and future external sources.
