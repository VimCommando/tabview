## Context

The live startup path reads an entire `InputSource`, decodes and parses it as CSV-like data, and passes `Vec<Vec<String>>` to `TableView::classify`. `TableView` then removes a possible first-row header, infers columns from all rows, stores all rows directly, and implements filters, sort, widths, search helpers, skip helpers, and clipboard helpers against the materialized vectors.

`src/table/mod.rs` already contains `TableStore`, `InMemoryTable`, and a prototype `LazyFileTable`, but the trait does not expose row access and the prototype fully indexes a delimited file during open. The active large-file design therefore needs to address both the unused store boundary and the CSV assumptions above it.

JSON and NDJSON introduce a second axis. Their columns come from object structure, native values have types and nulls, nested tables may begin below the document root, and a bounded schema scan may discover additional columns later. SQLite is intentionally deferred, but its future catalog/schema/row behavior motivates keeping format discovery, table definition, and row storage separate now.

## Goals / Non-Goals

**Goals:**

- Open every supported input through a format adapter that returns an explicit table definition and row store.
- Preserve existing delimited behavior while removing header and column discovery from `TableView`.
- Preserve raw value kinds and stable source column identity independently from view formatting.
- Preserve opaque row identity within an opened source generation so query results can track selection and reject stale state.
- Add JSON/NDJSON input, embedded JSON table selection, bounded or full schema discovery, compact column labels, and append-only late columns.
- Make the TUI and existing operations work through in-memory and incremental stores.
- Represent sort and filter as stable, source-neutral query specifications with canonical local execution and an optional exact-execution capability for future stores.
- Keep initial work bounded by format policy unless the user requests a full scan or full-table operation.
- Leave a source/relation boundary that a later SQLite adapter can implement without restructuring the view again.

**Non-Goals:**

- SQLite connections, table selection UI, SQL generation, or a database operation-pushdown implementation. This change defines only the source-neutral operation contract and local fallback that a later adapter can implement.
- JSONPath expressions, predicates, recursive descent, joins, array explosion, or arbitrary data transformation. JSON starting paths use RFC 6901 JSON Pointer only.
- Editing or persistent mutation of source data.
- An asynchronous runtime. Bounded synchronous chunks and explicit status are sufficient initially.
- Transparent random seeking for non-seekable input without spooling; stdin may materialize when required.
- Renaming previously displayed columns as provisional JSON schemas evolve.

## Decisions

### 1. Separate source resolution, table definition, and row storage

Use three related boundaries:

```text
InputSource + OpenOptions
          |
          v
    FormatResolver
          |
          v
    OpenedSource -------- list_relations() for future database sources
          |
          v
     OpenedTable
       |      |
       |      +-- Box<dyn TableStore>
       +--------- TableDefinition
```

Representative types are:

```rust
enum InputFormat {
    Auto,
    Delimited,
    Json,
    Ndjson,
}

struct OpenOptions {
    format: InputFormat,
    delimited: DelimitedOptions,
    json_path: Option<JsonPointer>,
    schema_scan: SchemaScan,
}

trait SourceAdapter {
    fn probe(&self, source: &InputSource, sample: &[u8]) -> ProbeResult;
    fn open(&self, source: InputSource, options: &OpenOptions) -> Result<OpenedSource>;
}

struct OpenedTable {
    generation: SourceGeneration,
    definition: TableDefinition,
    store: Box<dyn TableStore>,
}
```

`OpenedSource` exposes one implicit relation for delimited, JSON, and NDJSON inputs. Relation metadata reserves the source/table separation, while public multi-relation construction and selected-relation opening are explicitly deferred to the follow-on SQLite/database-source feature in the database worktree; no database adapter or relation selector is included here.

Alternative considered: one generic `FileParser` that returns rows. This keeps the current coupling between parsing, schema discovery, and materialization and cannot naturally represent multiple future relations, so it is rejected.

### 2. Resolve saved-view source options before opening the table

Saved views currently resolve columns only after a table exists. Source options such as format, JSON starting path, and schema scan policy must be known earlier. Opening becomes two-phase:

1. Parse CLI arguments and discover/select a saved view using the input filename.
2. Merge source options with precedence `explicit CLI > selected saved view > defaults`.
3. Validate cross-format option compatibility.
4. Resolve the format and open the initial table definition/store.
5. Resolve saved column, sort, filter, and display configuration against the initial schema.
6. Retain unmatched canonical structured-column configuration while the schema is provisional.

An explicit delimited-only CLI option such as `--delimiter` implies delimited format when format remains `auto`, preserving compatibility. It is an error when combined with explicitly selected JSON/NDJSON.

### 3. Use a first-class table definition

```rust
struct TableDefinition {
    columns: Vec<ColumnDefinition>,
    schema_state: SchemaState,
    relation: RelationMetadata,
}

struct ColumnDefinition {
    id: ColumnId,
    source_identity: ColumnSourceIdentity,
    display_name: String,
    source_type: LogicalType,
    type_origin: TypeOrigin,
}

enum SchemaState {
    Provisional,
    Complete,
}
```

Column identity is ordinal/stable-ID based and is never derived solely from display text. Delimited headers, canonical JSON pointers, generated array positions, and future database columns are represented as source identity variants. Rows similarly carry opaque `RowId` values; file adapters derive them from logical source-row position, while future adapters may choose another identity. `RowId`, `ColumnId`, and every derived query result are scoped to the `SourceGeneration` created when a relation is opened. `TableView` owns header visibility and rendered decorations, but the header cells are projections of column definitions rather than a data row stored separately in the view.

Reload creates a new generation. View and saved configuration are re-resolved through durable source identities, while row IDs and query result stores from the previous generation are discarded. An adapter must not combine bytes or rows from different detected source versions in one generation; if a seekable file is observably replaced, truncated, or changed incompatibly during later indexing, the operation fails without activating partial state and prompts a reload.

The delimited adapter retains the current compatibility heuristic and consumes the classified record before constructing its row store. JSON and NDJSON construct names from object paths and never remove a row to obtain a header.

Alternative considered: keep `Option<Vec<String>>` as a header beside inferred `Columns`. That duplicates names, cannot distinguish a JSON-derived schema from a consumed header record, and makes late schema updates fragile, so it is rejected.

### 4. Preserve typed raw cells

Rows use a typed cell representation before view formatting:

```rust
struct Row {
    id: RowId,
    cells: Vec<CellValue>,
}

enum CellValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Binary(Vec<u8>),
    Json(String),
}
```

Delimited fields enter as `Text`, including empty text. JSON scalars retain native kinds, nested arrays are retained as compact structured JSON, and missing object paths are `Null`. Binary is included for the stable prerequisite model even though current adapters do not produce it.

Source type metadata is distinct from the existing saved-view type interpretation and display formatting. Inferred JSON types widen monotonically as values arrive: null/unknown can adopt a concrete type, integer plus float widens to float, and incompatible concrete families widen to mixed metadata while individual `CellValue` kinds remain intact.

Alternative considered: stringify values at ingestion and attach a type hint. That loses the difference between null and empty text and forces native numeric and boolean operations to reparse presentation strings, so it is rejected.

### 5. Expand `TableStore` for partial access and schema deltas

The store boundary supports row access, incomplete counts, incremental indexing, scanning, and controlled materialization:

```rust
trait TableStore {
    fn row_count(&self) -> RowCount;
    fn row(&mut self, index: RowIndex) -> Result<Option<Row>>;
    fn ensure_indexed_through(&mut self, index: RowIndex) -> Result<IndexProgress>;
    fn index_and_scan_rows(&mut self, through: RowIndex, request: ScanRequest, visitor: &mut dyn RowVisitor) -> Result<IndexScanProgress>;
    fn scan_rows(&mut self, request: ScanRequest, visitor: &mut dyn RowVisitor) -> Result<ScanProgress>;
    fn materialize(&mut self) -> Result<InMemoryTable>;
    fn try_execute_query(&mut self, query: &TableQuery) -> Result<QueryExecution>;
}

struct IndexProgress {
    row_count: RowCount,
    schema_delta: SchemaDelta,
    bytes_scanned: u64,
}

enum RowCount {
    Exact(usize),
    AtLeast(usize),
    Unknown,
}
```

The table definition, not the store, is authoritative for current column count. Incremental stores return append-only schema deltas when discovery adds columns or widens source type metadata. Existing rows read through the expanded definition yield `Null` for newly appended columns. `scan_rows` supplies bounded progressive traversal and generic fold/reduction without forcing callers to clone the table; it is separate from query execution because it does not define a persistent result row set.

Small inputs and non-seekable streams may use `InMemoryTable`. Large seekable delimited inputs index CSV logical-record offsets. Large NDJSON inputs index complete document offsets. Large JSON array inputs index selected array-element boundaries while respecting strings and nesting. Implementations must not use naive brace or newline splitting.

Use `serde_json` as the JSON parser. This follows the streaming pattern already proven in ESDiag: `serde_json::StreamDeserializer` processes top-level/concatenated values, while custom Serde `Visitor` and `DeserializeSeed` implementations traverse outer maps/sequences, skip unrelated values with `IgnoredAny`, and emit selected entries without materializing their container. `serde_json::value::RawValue` preserves nested arrays or other structured cells without first building a full `Value` tree.

For NDJSON, use `Deserializer::from_reader(...).into_iter()`/`StreamDeserializer` and its consumed-byte offset to index complete documents. For a selected array inside a regular JSON document, use a path-aware visitor/seed to descend only through matching JSON Pointer segments and stream the target `SeqAccess`. Wrap the buffered source in a shared counting reader so the array-element seed can record logical start/end positions around each `next_element_seed` call. Validate recorded positions by seeking and independently reparsing indexed elements across whitespace, escaped strings, nested structures, and chunk boundaries. If Serde consumes separator lookahead, normalize the recorded boundary to leading JSON whitespace before reparsing; this remains an offset-adapter concern around `serde_json`, not a separate parser-selection question.

### 6. Keep lazy and schema-scan thresholds independent

```rust
const DEFAULT_LAZY_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;
const DEFAULT_SCHEMA_SCAN_BYTES: u64 = 100 * 1024 * 1024;

enum SchemaScan {
    Default,
    Full,
}
```

The lazy threshold chooses a storage strategy for adapters where source size is meaningful. The schema scan limit controls how much of a selected structured table is inspected before the initial schema is fixed. They remain distinct even while both default to 100 MiB.

Default JSON discovery scans selected logical rows until the selected payload bytes reach the limit, finishing the row that crosses it. If the selected table ends first, the schema is complete; otherwise it is provisional. A full scan indexes through the selected table's end without retaining every decoded row and marks the schema complete.

This design intentionally accepts up to 100 MiB of streaming parse work before the first stable frame. It provides substantially broader initial schema discovery without requiring full materialization. Status must identify full scans and other visibly long opening work.

### 7. Define JSON starting-path semantics

`--json-path` and saved-view `json_path` contain an RFC 6901 JSON Pointer.

- For a regular JSON document, the pointer is resolved once against the document root. A selected array supplies its elements as rows; a selected object supplies one row.
- For NDJSON, the pointer is resolved independently against each logical JSON document. The selected object or array is that document's row value; it does not multiply one NDJSON record into several rows.
- A missing pointer or selected scalar is a clear source-opening/indexing error.
- Column pointers are relative to the resulting row root, not prefixed with the table starting path. For `/hits/hits`, an Elasticsearch hit column can therefore be `/_source/user/id`, while `/hits/hits` remains separate source configuration.
- JSON Pointer escaping distinguishes keys containing `/` and `~`; display labels additionally use bracket notation or escaping where dot notation would be ambiguous.

Alternative considered: JSONPath. Its multiple dialects, recursive queries, filters, and multi-node results would turn source selection into a transformation language, so it is excluded from this change.

### 8. Flatten objects but retain nested arrays

Object-valued rows are recursively flattened to leaf paths. A scalar at `/a` and nested leaves such as `/a/b` may coexist across heterogeneous rows as distinct columns. Arrays nested inside object rows remain atomic structured JSON cells. Array-valued rows use generated positional column identities and labels.

This avoids unbounded column/cardinality expansion from nested arrays while supporting the common log/API-response shape. Array explosion can be considered later as an explicit transformation.

### 9. Separate canonical JSON identity from compact labels

Canonical JSON column identity is the case-sensitive RFC 6901 pointer relative to the row root. The initial display label is the shortest unique suffix of its path segments:

```text
/customer/profile/email  -> profile.email
/billing/email           -> billing.email
/created_at              -> created_at
```

Uniqueness is calculated over the initially discovered schema. Labels established at initial render are frozen. A late path receives the shortest non-conflicting suffix without renaming an existing column. First-seen path order is preserved, and late paths append on the right.

Column information shows canonical identity. Saved views match exact canonical JSON pointers case-sensitively first, then allow an unambiguous display-label fallback. Canonical configurations absent from a provisional schema remain pending. A `label` override changes only presentation.

Alternative considered: render full paths for every header. It is stable but wastes most terminal width. Leaf-only labels are compact but ambiguous. Shortest unique suffixes plus canonical identity provide both usability and precision.

### 10. Represent sort and filter as table queries with exact-or-fallback execution

`TableView` owns cursor, viewport, header visibility, column presentation, filters, sort configuration, and schema-derived view metadata, while rows come from its opened table/store.

Sort and filter are row-set operations whose operands happen to be columns; they are not behavior on `ColumnDefinition`. Represent active operation state as a source-neutral query keyed by stable `ColumnId` rather than display labels or transient visible ordinals:

```rust
struct TableQuery {
    filters: Vec<FilterSpec>,
    order_by: Vec<SortSpec>,
}

struct FilterSpec {
    column: ColumnId,
    mode: FilterMode,
    predicate: FilterPredicate,
}

struct SortSpec {
    column: ColumnId,
    mode: SortMode,
    direction: SortDirection,
    nulls: NullPlacement,
}

enum NullPlacement {
    First,
    Last,
}

enum QueryExecution {
    Executed(Box<dyn TableStore>),
    Unsupported,
}
```

`FilterPredicate` retains the existing text, regex, and numeric semantics, including whether matching observes raw values, rendered values, or both. `SortMode` retains the existing lexical, natural, numeric, and saved-view type-specific semantics. Multiple filters are combined using the current behavior, and `order_by` preserves the current multi-sort precedence.

Null placement is view operation configuration, not source schema. Saved views accept top-level `nulls: first|last` as the default for every sorted column and the same field inside a `columns` entry as an override. Resolution is `column nulls > view nulls > Last`. The resolved `NullPlacement` is copied into each `SortSpec`, making the complete query independent of later label or presentation lookup. Null placement is direction-independent: `first` stays first and `last` stays last for both ascending and descending sorts. Each key in a multi-column sort uses its own resolved policy.

Changing the effective policy for a column already present in `order_by` rebuilds and atomically re-executes the query. The column information editor exposes inherited/view-default, first, and last choices; serialization omits an inherited column value and writes explicit overrides. A late-discovered column receives pending saved configuration before it can participate in sorting.

The generic local executor is the semantic reference implementation:

- Sorting is stable. Rows equal under every active key retain their relative base-source order, located through `RowId`, independent of their order in the previously active result.
- `Null` is distinct from `Text("")` and uses the sort key's resolved first/last policy in both ascending and descending directions. Missing structured fields use `Null` and therefore follow the same rule. The built-in default is last.
- Native integers and floats participate directly in numeric comparison; textual cells retain the existing numeric and suffix parsing behavior. Numeric comparison uses a deterministic total float order for accepted floating-point values, and non-numeric values retain the existing mode-specific fallback order before nulls.
- Lexical comparison uses Rust string ordering without locale collation, natural comparison uses the existing tokenizer, and text filters remain case-sensitive substring matches.
- Regex filters use the Rust `regex` crate's syntax and Unicode behavior. Numeric predicates never match null or non-numeric cells. Text and regex predicates evaluate exactly the value domains recorded in `FilterPredicate`, including rendered values where current behavior requires them.
- `FilterMode::Out` is the logical negation of the corresponding match result, including for null and non-numeric values.

These rules are part of query compatibility. A future source executor must match each sort key's explicit null placement and stable tie order or return `Unsupported`.

Before consulting a store, the coordinator validates every `ColumnId`, predicate, comparison mode, and generation reference without changing active state. `TableStore::try_execute_query` has a default `Unsupported` implementation and distinguishes that capability result from `Result::Err`, which means execution failed. A store may return `Executed` only when it can preserve the entire query's canonical viewer semantics; the returned store is a generation-bound derived result row set with the same table definition. It may itself be incremental and report `Unknown` or `AtLeast` row count while producing rows lazily. Query correctness does not require materializing a source-executed result.

Unsupported execution is not an error: the coordinator fully indexes or materializes the base store and runs the generic local executor. Returning `Unsupported` may retain monotonic indexing or cache progress, but may not change base row order or active view/query state. Current delimited, JSON, and NDJSON stores use that fallback. Partial pushdown and SQL translation are deferred with SQLite.

This exact-or-fallback rule is important because a superficially similar source operator may not be equivalent to Tabview behavior. For example, text and regex filters may inspect rendered values, while natural, numeric-suffix, date, semantic-version, IP, and boolean sorting have viewer-defined comparisons. An adapter that cannot reproduce those semantics must return unsupported rather than an approximate result.

Query execution produces a derived logical result row set and never changes the base store's source order. `TableView` retains the base store plus the last successful result. Clearing sort/filter state re-executes the remaining query or returns to source order without reopening the source. Query state and replacement results are built separately and become active atomically; an execution failure preserves the prior query, row set, cursor, and viewport.

Before a successful replacement, the view captures the selected `RowId` and any marked `RowId`/`ColumnId`. If the selected row remains in the result, the cursor follows it to its new position. If filtering removes it, the cursor clamps the previous visible position into the new result. A mark remains associated with its row and column identities even while filtered out and becomes reachable again when a later query includes that row. Generation changes invalidate these row identities rather than accidentally targeting same-numbered rows in a new source.

Alternative considered: place `sort` and `filter` methods on `ColumnDefinition`. This confuses column metadata with row-set execution, cannot represent multi-column ordering or combined predicates, and gives a future database adapter no coherent whole-query planning boundary, so it is rejected.

### 11. Move `TableView` to store-backed operation categories

`TableQuery` is deliberately limited to persistent row membership (`filters`) and row ordering (`order_by`). Search and skip-to-change are progressive navigation commands over the active result and use bounded row scans; they do not create query clauses. Width calculation, numeric/type profiling, gradients, and identifier analysis are scan/fold reductions. They use `scan_rows` and may have sampled or exact policies, but they do not return or replace the active row set. Source-specific aggregate pushdown is not included in this change.

Operations remain divided by how much source access they require:

- **Viewport-local:** rendering indexed rows, cursor movement within the indexed range, current cell popup, table info from known state, raw/rendered current-cell yank.
- **Progressive:** navigation beyond the indexed range, forward search, and skip-to-change. These request bounded indexing until a result or selected-table end.
- **Full-table local fallback:** sort, complete filtering, exact max-content sizing, and exact auto-range/identifier profiling. These fully scan, index, or materialize with status when executed locally, build replacement state separately, and swap it into the view only after success. A source-executed query result may remain lazy.

Automatic column widths are sampled from the initial loaded rows and then frozen; later scrolling or forward indexing does not resize existing columns as a side effect. An automatic width is capped at 80 percent of the current terminal cell width, while explicit fixed widths and manual column growth remain uncapped by that viewport policy. Terminal resizing may reapply the viewport cap to the frozen sample without resampling later rows.

Large forward jumps use store scans in sequential batches. In particular, `G` may index through EOF to establish the exact last row, but a delimited store performs that indexing and row delivery in one forward parser pass: each logical record contributes its offset and decoded row together, and the view retains that decoded row instead of seeking and decoding it again. The operation stages index metadata until the source-stability check succeeds. It must not reopen and independently parse the source once per indexed row, parse the same remaining range once for indexing and again for loading, or rerun sampled width/profile inference over the growing prefix.

Helpers that currently build `visible_rows_vec`, `visible_raw_rows_vec`, or `search_rows_vec` for a single-cell or progressive command must be replaced by row/cell accessors and iterators/scans over the store. Rendering must accept `Unknown` and `AtLeast` counts.

For the initial implementation, adapters do not execute queries directly. Sort and filter reach the generic local executor through the same query contract that a later database store may implement exactly.

### 12. Preserve state and surface progress

The terminal session is entered before initial source opening and immediately draws a footer-only loading state containing `Loading <filename>`, where `<filename>` is the input source's user-facing display name. That message remains visible throughout format resolution, schema discovery, initial indexing, and view construction. The first successful table-data frame atomically replaces the loading state with the normal footer state; it must not be cleared merely because opening returned or a partial model exists. If opening fails before a data frame renders, terminal state is restored before the error is reported.

Subsequent indexing, schema scanning, and full-table operations report progress/status through the same application message path. Errors preserve the last valid definition, store, cursor, viewport, filters, and sort configuration whenever the failing action is non-fatal.

Reload repeats saved-view source-option resolution and adapter opening, creates a new source generation, then reapplies view state and query configuration by stable source column identity. Query result stores and row identities from the prior generation are discarded. A changed or provisional JSON schema may append/remove columns across reload; unmatched state is ignored with a warning rather than applied by stale ordinal.

## Risks / Trade-offs

- **[100 MiB schema discovery can delay first render]** -> Parse streaming without materializing rows, enter the terminal first and keep `Loading <filename>` visible until the first data frame, keep the limit centralized, and allow future tuning without coupling it to the lazy threshold.
- **[Serde visitors do not directly expose nested element offsets]** -> Pair the path-aware `DeserializeSeed` with a shared counting reader and verify every recorded offset through independent seek/reparse fixtures covering whitespace, escaped strings, nested arrays/objects, large surrounding metadata, and chunk boundaries.
- **[CSV records can span physical lines]** -> Continue using parser-provided logical record positions; never index delimited rows by raw newline.
- **[Late columns expand view metadata while running]** -> Permit append-only schema deltas, freeze existing IDs/names/order, extend all per-column vectors centrally, and test saved-view/presentation application on arrival.
- **[Typed cells touch sorting, filters, rendering, clipboard, and saved views]** -> Introduce a single raw/display conversion boundary and migrate operations incrementally under existing compatibility tests.
- **[Source execution could subtly disagree with local operation semantics]** -> Treat the generic local executor as canonical, require exact support for the complete query, distinguish unsupported from failure, and defer partial pushdown until equivalence can be tested.
- **[Equal-key ordering can vary across executors]** -> Require stable base-source tie order and make source executors return unsupported unless they can provide an equivalent deterministic tie-breaker.
- **[Derived state can outlive its source]** -> Scope row/column IDs and query results to a source generation, reject detected mid-generation source replacement, and re-resolve durable configuration on reload.
- **[Full-table operations remain expensive]** -> Make their cost explicit, preserve state until success, and avoid accidental full clones from local commands.
- **[Automatic format probing can misclassify ambiguous text]** -> Give explicit CLI selection highest precedence, let saved views select format, and preserve delimited implications from existing format-specific flags.
- **[Case-sensitive JSON paths differ from current header matching]** -> Prefer canonical pointers, allow label fallback only when unambiguous, and surface full identity in column information.
- **[Non-seekable streams cannot provide arbitrary lazy access]** -> Materialize or spool only when required and document the behavior; do not pretend filesystem-size policy applies to stdin.

## Migration Plan

1. Add source/schema/value types and adapters while keeping the delimited adapter backed by `InMemoryTable`; prove existing CSV behavior through compatibility tests.
2. Move saved-view discovery and source-option merging ahead of table opening, then apply column configuration after initial schema construction.
3. Rework `TableView` and UI rendering to consume `OpenedTable`/`TableStore`, replacing full-vector helpers used by local operations.
4. Introduce stable query specifications, generation-bound row identities, the exact-or-unsupported execution boundary, and the canonical local executor; migrate sort/filter state to derived logical results without mutating source order.
5. Implement incremental delimited storage and migrate progressive/full-table operations with status and failure-safe swaps.
6. Add JSON/NDJSON adapters, starting-path resolution, typed row flattening, schema scanning, compact labels, and append-only schema deltas.
7. Update saved-view schema/serialization, CLI documentation, fixtures, render tests, and large-file integration tests.

Each stage should leave the crate testable. If the migration must be rolled back before completion, the in-memory delimited adapter provides the compatibility fallback while incomplete adapters remain unreachable from format resolution.
