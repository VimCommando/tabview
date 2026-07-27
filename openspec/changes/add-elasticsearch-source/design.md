## Context

Tabview currently treats the positional CLI value as a local `PathBuf` or stdin marker. `SourceAdapter` opens it into `OpenedSource`/`OpenedTable`, `TableDefinition` describes a generation-scoped fixed schema, and `TableStore` supplies typed rows. SQLite added relation discovery, bounded source operations, query provenance, and revisioned asynchronous replacement, but its replacement contract assumes every query keeps the selected relation's schema.

Elasticsearch introduces two related changes. First, the positional target is a remote HTTP(S) endpoint rather than a file. Second, ES|QL is a complete pipeline language whose result columns can differ from the selected index mappings. A user query can select multiple indices and can add, remove, rename, reorder, aggregate, or retype columns. Mapping discovery is still valuable for generated queries and source editing, but returned ES|QL column metadata must define the rendered result.

The CLI should not acquire a separate family of endpoint-, index-, and query-specific options for every backend. `--format`, `--table`, `--query`, the positional target, and existing structured source operations must remain the shared vocabulary.

## Goals / Non-Goals

**Goals:**

- Accept remote URL targets without interpreting them as filesystem paths.
- Infer a format from an unambiguous URL scheme while retaining explicit format precedence.
- Add an optional Elasticsearch adapter using the official Rust client and the existing Tokio runtime.
- Discover selectable open non-hidden indices and data streams for interactive target selection.
- Use mappings and field capabilities as the base field catalog and ES|QL response metadata as the active result definition.
- Execute complete user ES|QL or generate bounded ES|QL from `--table`.
- Add a generic `--query` that also supports confined native SQLite SQL.
- Compose existing source filters, source sorting, and limits using adapter-native query syntax.
- Publish schema-changing native-query results atomically and preserve the last valid result on failure.
- Keep native query artifacts language-neutral and free of connection secrets.
- Reuse the existing table/view and output adapter paths.

**Non-Goals:**

- Elasticsearch document mutation, index administration, bulk APIs, or Query DSL search as an alternative primary query language.
- Parsing arbitrary ES|QL to recover all source indices for mapping discovery.
- Offering hidden, system, dot-prefixed, closed, or alias targets in the first picker.
- Remote Turso/libSQL execution; this change only reserves and resolves its scheme.
- Pagination beyond Elasticsearch's bounded ES|QL result contract.
- Exact total-hit counts for ES|QL results.
- Persisting credentials in saved views or adding source-specific credential CLI flags.
- Named connection profiles or per-endpoint credential configuration.
- Making source-native SQL/ES|QL comparison semantics equal Tabview's local view semantics.

## Decisions

### Parse a source target before format resolution

Replace the path-only input concept with a target enum conceptually equivalent to:

```rust
enum SourceTarget {
    Path(PathBuf),
    Stdin,
    Url(Url),
}
```

`file://` is normalized to `Path`; other valid URLs remain URLs. Safe display and persistence helpers remove URL userinfo and redact secret-bearing components before a target can enter diagnostics or saved views.

Format resolution uses this order:

1. Explicit CLI format.
2. Saved-view format.
3. Registered unambiguous scheme mapping.
4. Existing local extension, strong signature, and bounded content probing.

`libsql://` maps to SQLite even though the current SQLite adapter then reports that remote execution is not implemented. `http://` and `https://` are intentionally ambiguous and require explicit or saved `elasticsearch` selection. This separates identifying bytes or a service from deciding whether an adapter currently supports that target.

The alternative of treating every URL as `PathBuf` preserves fewer types but repeats fragile string-prefix checks and risks filesystem access against remote strings. Inferring Elasticsearch from HTTP(S) would prevent future remote JSON and other HTTP-backed adapters.

### Keep one generic CLI vocabulary

Add `elasticsearch` as a feature-gated input format and expose:

```text
tabview <target> [--format <format>] [--table <name> | --query <text>]
```

`--table` means a source-native selectable relation or target. `--query` means a complete native query interpreted by the resolved adapter. They conflict because allowing both would create two authorities for relation selection.

For SQLite, `--table` selects a table/view and `--query` is confined SQL. For Elasticsearch, `--table` selects an index/data stream and `--query` is complete ES|QL. File adapters reject both through capabilities rather than through source-specific CLI parsing.

If Elasticsearch has neither selection:

- Interactive and interactive-output execution opens the target picker.
- Direct output fails before stdout and requires `--table` or `--query`.

The alternative of `--index`, `--esql`, endpoint, or credential flags would duplicate concepts and make every future adapter expand the CLI.

### Use the structured resolve-index API for the picker

Elasticsearch discovery calls the official client's resolve-index API for `*` with open wildcard expansion. The response already separates indices, aliases, and data streams. The adapter retains:

- open non-hidden indices whose names do not begin with `.`;
- non-hidden data streams whose names do not begin with `.`.

It omits aliases and ignores data-stream backing indices as independent picker entries. Results are sorted deterministically and displayed in Indices and Data streams sections.

Discovery policy applies only to picker population. An explicit `--table` value is not resolved, classified, or checked against the picker catalog before use. The adapter safely renders it as the generated ES|QL `FROM` target and lets mappings, field capabilities, and ES|QL execution determine whether Elasticsearch accepts it. As a result, an explicitly entered alias, wildcard, or other valid `FROM` target works even though aliases are never offered by the picker.

The CAT indices API was considered because it can return a compact name list, but Elastic documents CAT APIs as human-oriented rather than application contracts and they can require broader monitoring privileges. A separate data-stream request was also considered; resolve-index supplies both resource kinds in one typed response with `view_index_metadata`.

### Separate the mapping catalog from the active result definition

When `--table` or the picker selects one target, the adapter obtains:

1. mappings for complete per-index/backing-index metadata;
2. field capabilities for the merged queryable view and conflicts.

This produces an Elasticsearch field catalog used by source configuration, completion, type-aware operator availability, and diagnostics. It is not installed directly as the active `TableDefinition`.

The successful ES|QL response `columns` array defines the active table. Each result column retains its raw ES|QL type and ordinal. A best-effort link to one mapping field is stored only when name and type lineage are unambiguous. Query-only startup without `--table` skips mapping discovery and builds directly from response columns; Tabview does not implement a partial ES|QL parser that would fail on patterns, subqueries, joins, or future syntax.

Schema-changing source replacement returns a complete result bundle:

```rust
struct SourceResult {
    definition: TableDefinition,
    store: Box<dyn TableStore>,
    extent: SourceResultExtent,
    query: Option<NativeQueryArtifact>,
}
```

The coordinator publishes the bundle atomically. Equal compatible definitions may retain a generation; changed column names, order, count, or types create a new generation and remap only compatible durable view configuration.

### Represent native query input separately from structured operations

Extend the effective source request conceptually:

```rust
struct SourceRequest {
    generation: SourceGeneration,
    native_query: Option<String>,
    filters: Vec<SourceFilter>,
    order_by: Vec<SourceSort>,
    limit: NonZeroUsize,
}
```

The native text is opaque to the shared model. Each adapter owns validation, identifier resolution, parameter binding, and composition.

Elasticsearch composition is:

```text
<user ES|QL or generated FROM target METADATA _index, _id>
| <supported WHERE stages>
| <supported SORT stage>
| LIMIT <hard boundary>
```

TUI-generated identifiers and values use ES|QL parameters where supported and adapter-owned safe identifier construction otherwise. A user base query may already contain sorting, aggregation, or limiting; those stages remain inside the opaque base, and Tabview's final limit remains the hard result boundary.

SQLite composition treats a valid native row query as a derived input:

```sql
SELECT *
FROM (<native query>) AS "__tabview_source"
WHERE <bound source predicates>
ORDER BY <quoted result columns>
LIMIT ?
```

The adapter rejects source operations over duplicate or otherwise unaddressable result names rather than guessing. Generated table SQL continues through the same composer.

The alternative of a universal SQL-like AST would either reduce ES|QL to a small subset or leak language-specific constructs through shared types. Parsing and rewriting arbitrary user text was rejected for the same reason.

### Confine native SQLite SQL

`--query` expands SQLite from generated selects to user-supplied SQL, but does not relax its read-only contract. The private SQLite facade accepts exactly one prepared statement, verifies that it is read-only and row-producing through the strongest statement metadata exposed by the pinned engine, and executes it only after storage read-only flags and `PRAGMA query_only` are enabled and verified.

Statements that mutate, attach, detach, change pragmas or schema, contain executable trailing text, or have no result columns are rejected. Tests snapshot database and sidecar state across accepted and rejected queries. If the pinned Turso API cannot prove statement read-only status, implementation must add a narrow validation boundary or retain a conservative accepted query subset; it must not fall back to executing arbitrary text and trusting eventual failure.

Arbitrary query results do not inherit SQLite rowid or primary-key identity merely because columns have familiar names. Identity remains unavailable unless the adapter can prove it from prepared source metadata.

### Generalize query provenance

Replace SQL-named provenance fields with:

```rust
enum NativeQueryLanguage {
    Sql,
    Esql,
}

struct NativeQueryArtifact {
    language: NativeQueryLanguage,
    logical: String,
    parameters: Vec<NativeQueryParameter>,
    copyable: String,
}
```

The logical artifact uses the configured limit, not a private extra-row probe. Saved views persist `source.query` plus structured filters/sort/limit; they do not persist a duplicated composed artifact. UI labels use the language rather than assuming SQL.

Transport credentials, authorization headers, URL userinfo, and secret environment values never enter this artifact.

### Use async remote jobs and bounded in-memory ES|QL results

The official client uses async HTTP on Tokio. Generalize `SourceQueryTask` into a task kind or boxed future that can run network futures directly and local blocking work through `spawn_blocking`. Revision coordination continues to activate only the latest result.

The first Elasticsearch store parses one structured JSON ES|QL response into a bounded in-memory table. ES|QL returns the result as one response rather than a cursor that matches the incremental SQLite store, so inventing incremental per-row HTTP access would add latency without reducing server result work.

Superseding a request aborts its client future when possible; a response that still completes is discarded by revision. This change does not use Elasticsearch's separate async-query API, expose server-side query IDs, or poll server-side progress.

### Preserve typed values and source types

Response JSON maps null, boolean, signed integral values, finite floating-point values, and strings to existing typed cells when lossless. Arrays and objects use the structured JSON cell representation. Values that do not fit an existing numeric representation remain lossless structured/text values with their raw ES|QL column type rather than being truncated.

Mapping and response type strings remain inspectable source metadata. Tabview does not invent exact local date, IP, spatial, unsigned, or union semantics; existing local modes may operate on rendered forms when explicitly selected.

### Model limit extent and partial execution independently

The default Elasticsearch source limit is 1,000. When below the endpoint's effective result ceiling, execution requests `N + 1`, retains at most `N`, and records complete versus limited. If an extra-row probe is unavailable, extent remains conservatively limited/unknown rather than claiming completion.

Elasticsearch `is_partial` is a separate flag. Interactive mode displays it. Batch output may emit an allowed partial result with a stderr warning; transport or query failure still produces no stdout. Local view filtering never refills from Elasticsearch.

### Use document metadata only when it survives the query

Generated target queries include `_index` and `_id` metadata. A unique pair supplies `StableRowIdentity::ElasticsearchDocument`. User ES|QL must request and retain those fields to receive the same behavior. Aggregations and other transformations normally remove identity and therefore reset cursor-following and marks across replacement.

The application does not infer identity from output position or arbitrary columns named `_id`.

### Keep credentials out of the generic CLI

The initial transport reads authentication and TLS material from environment-backed configuration:

- `ELASTIC_API_KEY`, or
- `ELASTIC_USERNAME` plus `ELASTIC_PASSWORD`,
- optional `ELASTIC_CA_CERT` for a custom certificate path.

API key and username/password modes conflict; incomplete credential pairs fail clearly. The endpoint remains the positional target. Environment variables are the only authentication and custom-CA configuration mechanism in this change. Named connection profiles and source-specific credential arguments are not introduced.

## Risks / Trade-offs

- [The official Rust client is still versioned as alpha] → Keep it behind a non-default optional feature, pin a verified compatible version, isolate client types inside the adapter, and cover supported Elasticsearch versions with integration tests.
- [Mappings for a wildcard or data stream can be large and conflicting] → Fetch them only for an explicit selected target, merge through field capabilities, retain conflicts, and allow query-only mode to rely on result metadata.
- [A complete ES|QL query can be expensive despite a final limit] → Keep replacement asynchronous, preserve the last valid result, expose pending state, and document that output limits do not necessarily bound upstream aggregation work.
- [Elasticsearch result ceilings can prevent `N + 1`] → Treat extent conservatively and never claim exact completeness without proof.
- [Schema changes can invalidate saved operations and presentation] → Publish a new generation atomically, remap only stable compatible identities, and surface stale configuration instead of matching by position.
- [Multivalued and source-specific types exceed the current scalar model] → Preserve arrays/objects losslessly and retain raw type metadata rather than coercing or dropping values.
- [Environment variables are process-global and shared across endpoints] → Document the one-process credential scope clearly and require callers switching clusters to set the appropriate environment for each invocation.
- [Native SQLite SQL broadens the attack and side-effect surface] → Require one proven read-only row statement inside storage and query-only confinement and verify physical/logical immutability in regression tests.
- [Picker discovery can expose more names than a user expects] → Request only open non-hidden resources, reject dot-prefixed names, honor Elasticsearch authorization, and avoid logging the complete catalog.
- [HTTP(S) is ambiguous] → Require explicit Elasticsearch format rather than probing arbitrary endpoints.

## Migration Plan

1. Introduce `SourceTarget` and scheme-aware format resolution while preserving every existing local invocation.
2. Generalize native query configuration, provenance, result bundles, and asynchronous tasks before adding the network adapter.
3. Add confined SQLite `--query` as a local proof of the generic contract.
4. Add the optional Elasticsearch client transport, discovery, mappings, ES|QL composer, typed response conversion, and in-memory store.
5. Integrate picker, Source Configuration, saved views, reload, output preparation, and documentation.
6. Validate feature-disabled builds, default builds, Elasticsearch-feature builds, physical SQLite read-only behavior, and live versioned Elasticsearch fixtures.

Rollback disables or removes the optional Elasticsearch feature and adapter. The source-target, native-query, language-neutral provenance, and schema-changing replacement contracts remain useful for SQLite and future remote sources. Saved views containing `source.format: elasticsearch` remain invalid in a build without that feature and fail with the normal unavailable-format diagnostic.
