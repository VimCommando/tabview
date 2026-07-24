## 1. Refactor saved and runtime configuration

- [ ] 1.1 Replace the flat saved-view model with root `name`/`filenames`, a `source` section, and a `view` section; pre-change flat YAML compatibility is not required.
- [ ] 1.2 Move format, table, JSON path, object mode, schema scan, source limit, source filters, and source sorting into the saved `source` model.
- [ ] 1.3 Move locale, null policy, columns, view filters, and view sorting into the saved `view` model.
- [ ] 1.4 Update parsing, semantic validation, schema generation, warnings, source-option merging, serialization, comment preservation, modal display, and fixtures for the nested structure.
- [ ] 1.5 Keep CLI source-option precedence field-specific within `source` and serialize interactive changes back into their owning layer.

## 2. Split source queries from view transforms

- [ ] 2.1 Replace the single persistent query state with `SourceQuery` and `ViewTransform` models keyed by stable `ColumnId`.
- [ ] 2.2 Define a small typed `SourceFilter` vocabulary for equality, inequality, ordered comparison, contains, prefix, and null tests using typed operands.
- [ ] 2.3 Define source-native sort keys separately from the existing natural, numeric, date, semantic-version, IP, boolean, rendered-value, and null-placement-aware view sorts.
- [ ] 2.4 Reuse the existing local filter and sort implementations exclusively for `ViewFilter` and `ViewSort`.
- [ ] 2.5 Apply operations in the fixed order SourceFilter → SourceSort → source limit → ViewFilter → ViewSort → search/render.
- [ ] 2.6 Ensure source-query changes replace the source result, while view changes only rebuild visible membership or order over the fixed source result.
- [ ] 2.7 Keep search and skip traversal within the locally transformed result without adding source-query clauses.
- [ ] 2.8 Add source-operation capability reporting so unsupported source filters or source sorts fail clearly without unbounded materialization.

## 3. Add bounded result and query-execution contracts

- [ ] 3.1 Add a positive source result limit to `SourceQuery`, using 1,000 as the SQLite default when saved `source.limit` is omitted.
- [ ] 3.2 Add complete-versus-truncated `ResultExtent` independently of row-store indexing progress and visible row count.
- [ ] 3.3 Detect truncation with a bounded extra row, retain at most the configured limit, and prevent navigation, reductions, or local materialization from crossing that boundary.
- [ ] 3.4 Extend successful source execution with query provenance containing the logical parameterized SQL, typed parameters, and a safely rendered copyable statement when the source provides one.
- [ ] 3.5 Replace the unconditional unsupported-query materialization fallback with explicit bounded-local, unsupported, and source-executed outcomes.
- [ ] 3.6 Add stable database row identity support for rowid and declared primary-key tuples, including a defined identity-reset outcome when a relation has no stable key.
- [ ] 3.7 Add a revisioned asynchronous source-query coordinator that retains the previous result, reports progress, atomically swaps ready results, and discards or cancels stale jobs.

## 4. Establish Turso and read-only access

- [ ] 4.1 Pin the candidate `turso` version with default features and the minimal Tokio features required for local asynchronous I/O.
- [ ] 4.2 Introduce a Tabview-owned SQLite facade that keeps the raw Turso connection private and exposes typed discovery, schema, source-query, and row-fetch operations without a general execute or arbitrary-SQL API.
- [ ] 4.3 Enable and verify `PRAGMA query_only=ON` immediately after connecting, failing source opening when confinement cannot be verified.
- [ ] 4.4 Centralize reviewed read-only SQL templates, typed parameter binding, identifier quoting, and copyable SQL rendering.
- [ ] 4.5 Add a logical snapshot harness proving mutation attempts are rejected and supported actions do not change schema or table contents.
- [ ] 4.6 Record release binary size, compile time, allocator configuration, and platform effects for Turso's default FTS and mimalloc features, and document that Turso FTS does not make existing SQLite FTS virtual tables selectable.

## 5. Add SQLite discovery, metadata, and selection

- [ ] 5.1 Add `InputFormat::Sqlite`, explicit `--format sqlite`, and bounded `SQLite format 3\0` signature probing before text decoding.
- [ ] 5.2 Add public `--table <name>` and saved `source.table`, map them to internal relation selection, and give CLI selection precedence.
- [ ] 5.3 Reject SQLite stdin and remote URLs and validate text-only options after source resolution.
- [ ] 5.4 Extend `OpenedSource` with a relation catalog whose entries can be selectable or unavailable with an actionable reason, plus an optional selected table and source-owned lazy relation opener without opening every relation.
- [ ] 5.5 Classify ordinary tables, ordinary views, virtual tables, shadow tables, and internal objects; make ordinary tables selectable, capability-probe ordinary views with a non-mutating zero-row query, exclude virtual tables from selection, and omit shadow/internal objects.
- [ ] 5.6 Implement explicit, automatic-single-selectable, missing, unsupported virtual, incompatible view, multiple-selectable-awaiting-selection, no-selectable, and empty discovery outcomes.
- [ ] 5.7 Add the “Select table” startup modal, show incompatible views disabled with reasons when the modal is required, bypass it for resolved or sole selectable candidates, open only a confirmed selectable relation, and cancel cleanly.
- [ ] 5.8 Make selection execution-mode aware: direct batch output fails without writing stdout when multiple selectable candidates remain, while interactive and interactive-export modes use the modal.
- [ ] 5.9 Add relational column identity alongside existing structured-path and object-key identities, carrying relation, ordinal, and source name with deterministic duplicate-name saved keys.
- [ ] 5.10 Build complete table column metadata without consuming a row as a header; preserve raw SQLite declarations, derive initial hints from ordered affinity rules, leave NUMERIC/no-type/`ANY` and untyped view expressions unknown, derive view columns from prepared result metadata, and mark view row identity unavailable.

## 6. Compile and execute bounded SQLite source queries

- [ ] 6.1 Compile the selected relation, source filters, source sort keys, stable tie-breaker, and limit into one logical parameterized `SELECT`, with any private `N + 1` truncation probe kept out of the reusable query artifact.
- [ ] 6.2 Map supported source predicates directly to SQLite-native `WHERE` behavior without attempting view-semantic equivalence.
- [ ] 6.3 Apply SQLite-native source ordering before the limit and append a stable hidden identity tie-breaker for ordinary tables where available; do not infer identity for views.
- [ ] 6.4 Implement `TursoTableStore` around one active result stream, cached typed rows, schema deltas, result extent, row identity, and query provenance.
- [ ] 6.5 Convert Turso null, integer, real, text, and blob values directly into typed `CellValue` variants.
- [ ] 6.6 Implement bounded row access, forward fetch, cached backward access, limited materialization, and deterministic resource cleanup.
- [ ] 6.7 Reapply the current `ViewTransform` after every successful source-result replacement without expanding the source limit.
- [ ] 6.8 Preserve cursor and marks through stable row identity where possible and reset identity-dependent state explicitly where not possible.

## 7. Add file-source filtering and layered UX

- [ ] 7.1 Add streaming whole-record and column source-filter support for applicable delimited, JSON, and NDJSON adapters over decoded logical records.
- [ ] 7.2 Ensure delimited header classification and structured schema discovery occur correctly when source filters exclude data records.
- [ ] 7.3 Report source sorting as unavailable for adapters that cannot perform it within their supported resource contract.
- [ ] 7.4 Add a Source Configuration modal for source identity, limit and extent, source filters, source sort, adapter capabilities, and query plan or SQL; stage edits and apply the complete draft with at most one asynchronous replacement.
- [ ] 7.5 Add a source-neutral View Configuration modal for global view filters, view sort precedence, null behavior, and column presentation, with local updates that never re-query the source.
- [ ] 7.6 Keep Column Info as a contextual current-column editor with View filter and View sort controls, synchronize those controls bidirectionally with global View Configuration, show source-operation summaries read-only, and keep all existing quick filter/sort commands view-scoped.
- [ ] 7.7 Add editable source limit settings without automatically increasing the limit after view filtering.
- [ ] 7.8 Display visible count, fetched source count, and complete/limited extent distinctly.
- [ ] 7.9 Add a query modal or equivalent action that displays and copies SQLite source SQL and identifies active local view transforms that are not represented by it.
- [ ] 7.10 Reopen the selected relation and reapply saved source and view state on reload while preserving the last valid result on failure.
- [ ] 7.11 Route SQLite through the existing output adapter preparation path so complete-row traversal, width profiling, and serialization stop at the bounded source result.
- [ ] 7.12 Make direct batch startup await its initial source query, and make post-interactive export await the latest requested query revision without emitting partial stdout.

## 8. Verify and document

- [ ] 8.1 Add nested saved-view schema, parser, merge, serialization, comment, and round-trip tests for every `source` and `view` field.
- [ ] 8.2 Add layered-operation tests proving source operations precede the limit, view operations follow it, view filtering never refills, and search remains local.
- [ ] 8.3 Add source-capability and file-record-filter tests, including quoted multi-line CSV records.
- [ ] 8.4 Add SQLite compiler tests for every predicate, sort, null, quoting, parameter, stable-key, and limit case plus copyable SQL output.
- [ ] 8.5 Add SQLite fixtures for empty and multiple relations, ordinary tables, compatible and incompatible ordinary views, virtual tables using FTS5, RTree, and an unavailable module, virtual shadow tables, internal objects, quoted and duplicate names, rowid, composite primary keys, `WITHOUT ROWID`, STRICT tables, raw declared types, every SQLite affinity and value kind, misleading type spellings, contradictory runtime storage classes, generated columns, and data-dependent unsupported constructs.
- [ ] 8.6 Add store and coordinator tests for bounded first render, chunked fetch, truncation detection, stale-query replacement, failure preservation, cancellation, identity continuity, and cleanup.
- [ ] 8.7 Add logical read-only regression tests for discovery, modal selection, source and view operations, SQL output, reload, and close across rollback-journal and WAL fixtures.
- [ ] 8.8 Add render and interaction tests for selectable and disabled table-picker entries and their diagnostics; separate Source and View configuration modals; source draft/apply/cancel and capability states; view-only quick commands; bidirectional Column Info/View filter and sort synchronization; raw SQLite declarations and logical hints in Column Info; read-only source summaries; query progress; complete/limited extent; visible/source counts; SQL provenance; typed values; and errors.
- [ ] 8.9 Add direct and post-interactive output tests for sole-table selection, ambiguous-table failure, saved selection, bounded completion, view transforms, query failure, clean stdout, and broken-pipe handling.
- [ ] 8.10 Run formatting, Clippy, all-feature tests, release builds, and project-supported platform checks.
- [ ] 8.11 Update README and internal documentation with nested saved views, layered operations, source limits, SQL output, batch table selection, compatible-view probing, virtual-table exclusions, read-only guarantees, Turso FTS versus SQLite FTS compatibility, mimalloc, compatibility limits, and remote-access non-goals.
