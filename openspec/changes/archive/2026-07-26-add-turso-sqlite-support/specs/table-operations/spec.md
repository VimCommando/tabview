## MODIFIED Requirements

### Requirement: Operations over partial stores
Table operations SHALL run against an explicit active source result. Source operations MAY query or incrementally scan the underlying source up to their configured limit; view operations, search, navigation, and reductions SHALL remain bounded by the active source result unless the user explicitly changes the source query.

#### Scenario: Search incrementally reaches a later source-result row
- **WHEN** the next search result lies beyond the indexed portion of the active source result
- **THEN** search indexes forward until it finds the matching cell or reaches the end of that bounded result

#### Scenario: View filter requires complete result scan
- **WHEN** a view filter uses the generic local executor
- **THEN** the viewer may scan or materialize the complete active source result but does not read beyond its source-query boundary

#### Scenario: Source query result is incremental
- **WHEN** a store executes a source query and can stream its bounded result
- **THEN** the viewer may present rows incrementally while preserving the query limit and result-extent metadata

#### Scenario: Operation failure preserves state
- **WHEN** source execution or local view materialization fails
- **THEN** the previously successful source result, view order, cursor, filters, and sort state remain valid

#### Scenario: Current-cell yank remains local
- **WHEN** a user yanks the current raw or rendered cell
- **THEN** the viewer does not clone, materialize, or fetch unrelated rows

### Requirement: Source-neutral table queries
The system SHALL model persistent table operations as a source query followed by a view transform. Both layers SHALL use stable column identity independently of source format, display label, and visible position, while allowing the source layer to expose only operations its adapter can execute natively.

#### Scenario: Column label changes
- **WHEN** a label is overridden or a visible column moves while either layer references that column
- **THEN** the operation continues to reference the same stable source column

#### Scenario: Multiple operation clauses
- **WHEN** a layer contains multiple filters and sort keys
- **THEN** it preserves that layer's filter-combination behavior and multi-sort precedence in one complete request

#### Scenario: Layer ordering
- **WHEN** both layers are active
- **THEN** source filters and source sort run before the source limit, and view filters and view sort run afterward

#### Scenario: Structured column identity
- **WHEN** an operation references a structured or relational column
- **THEN** it resolves stable source identity rather than executing against the display label

### Requirement: Canonical execution and store fallback
The system SHALL use the generic local executor as the canonical implementation of view filters and view sorts. A source query SHALL use adapter-defined native semantics, and unsupported source operations SHALL NOT fall back to unbounded local materialization.

#### Scenario: Operation validation precedes execution
- **WHEN** either layer contains a stale or unknown column ID, invalid predicate, unsupported mode, or wrong source-generation reference
- **THEN** validation fails before execution and the active result and view remain unchanged

#### Scenario: Source store does not support an operation
- **WHEN** a store reports a requested source filter or source sort as unsupported
- **THEN** the viewer reports the capability limitation and retains the previous successful source query

#### Scenario: View operation executes locally
- **WHEN** a view filter or view sort is applied to any source
- **THEN** the generic local executor applies it to the active bounded source result

#### Scenario: File source capability
- **WHEN** a file adapter supports a source filter over logical records
- **THEN** it may execute that filter before the limit while continuing to use the canonical view executor for view operations

#### Scenario: Execution fails
- **WHEN** a store accepts a source query but execution fails, or local view execution fails
- **THEN** the error is reported and the previously successful operation state, result, cursor, and viewport remain active

### Requirement: Deterministic typed operation semantics
The system SHALL use one canonical comparator and predicate behavior for view operations across all sources. Source operations SHALL use documented source-native typed semantics and SHALL NOT be required to reproduce advanced view behavior such as rendered-value matching, Rust regex, natural sort, or Tview-specific numeric parsing.

#### Scenario: Equal view-sort keys
- **WHEN** two rows compare equal under every active view-sort key
- **THEN** their relative order matches active source-result order

#### Scenario: Default view null placement
- **WHEN** null and non-null cells are view-sorted without view or column null-placement configuration
- **THEN** null cells appear after non-null cells in either direction and remain distinct from empty text

#### Scenario: Typed view comparison
- **WHEN** a view operation receives native integer, floating-point, text, boolean, blob, or null cells
- **THEN** it applies the canonical local typed behavior without converting source semantics into SQL

#### Scenario: Canonical view text and regex behavior
- **WHEN** view text, lexical, natural, or regex behavior is evaluated
- **THEN** it uses the existing local case sensitivity, string ordering, natural tokenizer, Rust `regex` behavior, and requested raw or rendered domain

#### Scenario: Native source semantics differ
- **WHEN** SQLite collation, null placement, type affinity, or comparison semantics differ from Tview's view semantics
- **THEN** the source operation uses SQLite behavior and the UI identifies it as a source operation

### Requirement: Derived query results preserve source order
The system SHALL apply view filters and view sorts to a derived logical row set without mutating active source-result order, and SHALL replace the source result only when a source operation changes.

#### Scenario: View sort is cleared
- **WHEN** the user clears all view-sort keys and no view filters remain
- **THEN** rows return to active source-result order without reopening the source

#### Scenario: Source sort is cleared
- **WHEN** the user clears a source-sort key
- **THEN** the system executes a replacement source query and then reapplies the current view transform

#### Scenario: One view operation remains active
- **WHEN** the user clears one view sort or filter while other view clauses remain
- **THEN** the remaining view transform atomically replaces the previous derived result

#### Scenario: Replacement construction fails
- **WHEN** source execution, indexing, materialization, or local view execution fails before a replacement is complete
- **THEN** the previous source result and derived view remain unchanged

### Requirement: Query transitions preserve row identity
The viewer SHALL track cursor selection and marks by stable row and column identity across successful view transitions and compatible source-query replacements when the source provides stable row identity.

#### Scenario: Selected row moves after view sorting
- **WHEN** a successful view sort moves the selected row
- **THEN** the cursor follows that row identity and retains the selected column identity when visible

#### Scenario: Selected row is filtered out
- **WHEN** either filtering layer excludes the selected row
- **THEN** the viewer clamps the previous visible position into the new result

#### Scenario: Marked row temporarily leaves the view
- **WHEN** a view filter excludes a marked row
- **THEN** its mark remains associated with the stable row identity and becomes reachable again if a later view includes it

#### Scenario: Source has no stable row identity
- **WHEN** a source-query replacement cannot correlate rows with the prior result
- **THEN** row-bound cursor following and marks are reset rather than attached by result position

#### Scenario: Generation changes
- **WHEN** reload opens a new source generation
- **THEN** old row identities and marks are invalidated

### Requirement: Operation categories remain distinct
The system SHALL keep source queries, view transforms, progressive navigation, and scan or reduction operations distinct.

#### Scenario: Source query is persistent
- **WHEN** source filters, source sort, or source limit are active
- **THEN** they determine persistent membership, ordering, and maximum size of the active source result

#### Scenario: View transform is persistent
- **WHEN** view filters or view sort are active
- **THEN** they determine persistent membership and ordering only within the active source result

#### Scenario: Search and skip remain progressive
- **WHEN** search or skip-to-change traverses the active view
- **THEN** it uses bounded row scans without adding a source or view filter or sort clause

#### Scenario: Column analysis is a reduction
- **WHEN** width calculation, profiling, range analysis, or identifier analysis inspects many rows
- **THEN** it uses sampled or exact scan/fold behavior over the active source result without replacing it

#### Scenario: View does not refill source result
- **WHEN** view operations reduce the visible row count below the source limit
- **THEN** the viewer does not request additional source rows automatically

### Requirement: Sort persistence in saved views
The system SHALL serialize active source sort and view sort as separate ordered lists under `source.sort` and `view.sort`.

#### Scenario: Persist source sort
- **WHEN** a source sort is active and the user opens the saved-view modal
- **THEN** generated YAML includes its source column, direction, and source-supported kind under `source.sort`

#### Scenario: Persist view sort
- **WHEN** a view sort is active and the user opens the saved-view modal
- **THEN** generated YAML includes its source column, direction, and canonical local kind under `view.sort`

#### Scenario: Restore layered sort
- **WHEN** a saved view contains both lists and their source columns exist
- **THEN** source sort is applied before the source limit and view sort is applied to the resulting rows

#### Scenario: Search is not persisted
- **WHEN** a search query is active and the user opens the saved-view modal
- **THEN** generated YAML omits the search query

## ADDED Requirements

### Requirement: Source and View configuration modals
The interactive runtime SHALL provide separate Source and View configuration modals. Source configuration SHALL own construction of the bounded source result and MAY vary by adapter capability; View configuration SHALL summarize source-neutral local transformation and provide common local actions without changing the source result.

#### Scenario: Open Source configuration
- **WHEN** the user opens Source configuration
- **THEN** it shows source identity, source limit and result extent, source filters, source sort, capability status, and query provenance when the adapter provides it

#### Scenario: SQLite Source configuration
- **WHEN** the active source is SQLite
- **THEN** Source configuration offers supported SQLite-native predicates and sorting and shows the generated SQL representation

#### Scenario: File Source configuration
- **WHEN** the active source is delimited, JSON, or NDJSON
- **THEN** Source configuration offers supported logical-record filters, omits SQL, and explains when source sorting is unavailable

#### Scenario: Apply Source draft
- **WHEN** the user changes multiple source filters, sort keys, or the limit and confirms Apply
- **THEN** the complete draft is validated and starts at most one asynchronous source-query replacement

#### Scenario: Cancel Source draft
- **WHEN** the user cancels Source configuration
- **THEN** no draft source operation is applied and the active result remains unchanged

#### Scenario: Source replacement is pending
- **WHEN** an applied source draft is still executing
- **THEN** the previous successful result remains visible and Source configuration exposes pending or failure state

#### Scenario: Open View configuration
- **WHEN** the user opens View configuration for any supported source
- **THEN** it summarizes the active view filters, ordered sort keys, and view-wide null placement and offers actions to clear operations, toggle null placement, or open Column Info

#### Scenario: Clear View operations
- **WHEN** the user clears filters and sorts through View configuration
- **THEN** the local view is recomputed over the fixed active source result without re-querying the source

#### Scenario: Toggle View null placement
- **WHEN** the user toggles view-wide null placement through View configuration
- **THEN** active local sorts use the new policy without re-querying or expanding the source result

#### Scenario: Same column in both scopes
- **WHEN** a column participates in a source filter or sort
- **THEN** the user may independently configure a view filter or sort for that column

#### Scenario: Quick filter and sort commands
- **WHEN** the user invokes an existing current-column filter prompt or sort shortcut
- **THEN** it changes View configuration and never implicitly starts a source query

#### Scenario: Column Info applies a View filter
- **WHEN** the user adds, edits, or clears a filter for the current column through Column Info
- **THEN** the corresponding `ViewFilter` changes and the View Configuration summary reflects that change

#### Scenario: Column Info applies a View sort
- **WHEN** the user changes the current column's sort through Column Info
- **THEN** the corresponding `ViewSort` changes using established View sort precedence and the View Configuration summary reflects that change

#### Scenario: Open Column Info from View configuration
- **WHEN** the user opens Column Info from View configuration
- **THEN** Column Info displays and edits the current column's active View operation state rather than maintaining a separate copy

#### Scenario: Source operation in Column Info
- **WHEN** the current column participates in a source filter or source sort
- **THEN** Column Info may summarize that source operation but requires Source Configuration to edit it

#### Scenario: Saved View modal remains distinct
- **WHEN** the user opens the Saved View YAML modal
- **THEN** it serializes current runtime Source and View state without replacing either runtime configuration modal

### Requirement: SQLite source SQL output
When a SQLite source query is active, the user SHALL be able to inspect and copy the final source SQL together with its bound values or an equivalent executable SQL representation.

#### Scenario: Source operations change
- **WHEN** a source filter, source sort, table selection, or source limit changes
- **THEN** the displayed SQL is regenerated from the complete active source query

#### Scenario: View operations change
- **WHEN** only a view filter, view sort, or search changes
- **THEN** the source SQL remains unchanged and the output identifies those operations as local view behavior

#### Scenario: Identifier and value safety
- **WHEN** table names, column names, or filter values require quoting
- **THEN** identifiers are quoted by the SQL renderer and values remain bound parameters in execution metadata
