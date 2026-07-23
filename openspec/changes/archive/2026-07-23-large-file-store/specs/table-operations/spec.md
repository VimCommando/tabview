## MODIFIED Requirements

### Requirement: Header row behavior
The system SHALL render and toggle a fixed header from source column definitions while preserving compatible delimited first-row classification and the selected data cell where possible.

#### Scenario: Non-numeric delimited first row
- **WHEN** a multi-row delimited table has a first row with no numeric cells and no explicit header policy overrides classification
- **THEN** the delimited adapter consumes the first row as column names and shows those names as the fixed header by default

#### Scenario: Structured source columns
- **WHEN** JSON, NDJSON, or another structured adapter defines source columns without a header record
- **THEN** the viewer renders those column display names without removing the first data row

#### Scenario: Headerless generated columns
- **WHEN** a source has no named columns and the adapter generates stable column definitions
- **THEN** the adapter can keep the generated header hidden by default while retaining column identity

#### Scenario: Toggle header
- **WHEN** a user presses `t` for a table with renderable column definitions
- **THEN** the fixed header is toggled on or off while preserving the selected data cell where possible

## ADDED Requirements

### Requirement: Operations over partial stores
Existing table operations SHALL preserve their user-facing results when the table uses an incremental store, subject to explicit status while full-table work is performed.

#### Scenario: Search incrementally reaches a later row
- **WHEN** the next search result lies beyond the indexed range
- **THEN** search indexes forward until the matching cell is found or the selected table ends

#### Scenario: Local filter requires complete source scan
- **WHEN** an active filter falls back to the generic local executor
- **THEN** the viewer performs controlled full indexing, scanning, or materialization before presenting the final filtered result

#### Scenario: Source filter result is incremental
- **WHEN** a store executes a complete filter query with exact semantics
- **THEN** the viewer can present its derived result with an unknown or at-least-known row count and request later result rows incrementally

#### Scenario: Sort failure preserves order
- **WHEN** full indexing or materialization for a sort fails
- **THEN** existing row order, cursor, viewport, filters, and sort state remain valid

#### Scenario: Current-cell yank remains local
- **WHEN** a user yanks the current raw or rendered cell from an indexed row
- **THEN** the viewer does not clone or materialize unrelated rows

### Requirement: Source-neutral table queries
The system SHALL represent active sort and filter operations as a table query whose column operands use stable column identity independently of source format, display label, and visible column position.

#### Scenario: Column label changes
- **WHEN** a column label is overridden or a visible column moves while a sort or filter references that column
- **THEN** the operation continues to reference the same stable source column

#### Scenario: Multiple operation clauses
- **WHEN** a query contains multiple filters and sort keys
- **THEN** it preserves existing filter-combination behavior and multi-sort precedence in one complete operation request

#### Scenario: Structured column identity
- **WHEN** a query references a JSON or NDJSON column
- **THEN** the query resolves its stable column ID from the canonical row-relative source identity rather than executing against the compact display label

### Requirement: Canonical execution and store fallback
The system SHALL define generic local query execution as the canonical sort/filter behavior and SHALL allow a store to execute a complete query only when it can preserve those semantics exactly.

#### Scenario: Query validation precedes execution
- **WHEN** a query contains a stale or unknown column ID, invalid predicate, unsupported value domain, comparison mode, or source-generation reference
- **THEN** validation fails before store execution and the active query result and view state remain unchanged

#### Scenario: Store does not support a query
- **WHEN** a store reports that a complete query is unsupported
- **THEN** the viewer treats that response as a capability result, fully indexes or materializes under status, and executes the query with the generic local executor

#### Scenario: Operation cannot be translated exactly
- **WHEN** a source executor cannot reproduce any requested raw/rendered filter behavior or comparison mode exactly
- **THEN** it reports the complete query as unsupported instead of returning an approximate result

#### Scenario: Current file-backed stores execute locally
- **WHEN** a sort or filter is applied to a delimited, JSON, or NDJSON store in this change
- **THEN** the generic local executor determines the result and no source-specific pushdown is required

#### Scenario: Source execution fails
- **WHEN** a store accepts a query but execution fails
- **THEN** the error is reported and the previously successful query result, operation configuration, cursor, and viewport remain active

#### Scenario: Unsupported execution retains only internal progress
- **WHEN** a store returns unsupported after performing capability checks
- **THEN** it may retain valid monotonic indexing or cache progress but does not alter base source order, active query configuration, or the active result

### Requirement: Deterministic typed operation semantics
The system SHALL use one canonical comparator and predicate behavior across local execution and any store-executed query.

#### Scenario: Equal sort keys
- **WHEN** two rows compare equal under every active sort key
- **THEN** their relative order matches base source order regardless of the previously active result order

#### Scenario: Default null placement
- **WHEN** null and non-null cells are sorted without view or column null-placement configuration
- **THEN** null cells appear after non-null cells in either direction and remain distinct from empty text

#### Scenario: Typed numeric comparison
- **WHEN** a numeric operation receives native integer or floating-point cells, numeric textual cells, non-numeric cells, and nulls
- **THEN** native numbers compare without presentation parsing, numeric text uses the existing numeric/suffix rules, non-numeric values use the canonical fallback order, and nulls sort last or fail to match a numeric predicate

#### Scenario: Canonical text and regex behavior
- **WHEN** text, lexical, natural, or regex behavior is evaluated
- **THEN** it uses the canonical local case sensitivity, non-locale Rust string ordering, natural tokenizer, Rust `regex` syntax/Unicode behavior, and the raw/rendered domains recorded by the predicate

#### Scenario: Filter-out negates the match
- **WHEN** a filter-out predicate is evaluated for any cell kind
- **THEN** its result is the logical negation of the corresponding filter-in match

#### Scenario: Source semantics differ
- **WHEN** a store cannot reproduce null placement, stable tie order, value-domain behavior, collation, regex, numeric, or type-specific comparison semantics exactly
- **THEN** it reports the complete query as unsupported

### Requirement: Configurable sort null placement
The viewer SHALL support direction-independent `first` or `last` null placement as a view-wide sorting default with an optional per-column override, and SHALL include the resolved policy in every sort key.

#### Scenario: View-wide nulls first
- **WHEN** a view configures `nulls: first` and a sorted column has no override
- **THEN** null cells sort before non-null cells for both ascending and descending direction

#### Scenario: Column overrides view default
- **WHEN** a view configures `nulls: first` and the sorted column configures `nulls: last`
- **THEN** that column's null cells sort after non-null cells in both directions

#### Scenario: Multi-column null policies
- **WHEN** active sort keys have different effective null-placement policies
- **THEN** each comparison key applies its own resolved policy in multi-sort precedence order

#### Scenario: Null policy changes during active sort
- **WHEN** the effective null placement changes for a column already in the active sort query
- **THEN** the viewer rebuilds and atomically re-executes the query while preserving prior state on failure

#### Scenario: Textual null placeholder
- **WHEN** a delimited text cell contains a placeholder such as `null`
- **THEN** null placement does not treat it as `CellValue::Null`; existing type-specific placeholder ordering remains applicable

### Requirement: Derived query results preserve source order
The system SHALL apply sort and filter queries to a derived logical result row set without mutating the base store's source order.

#### Scenario: Sort is cleared
- **WHEN** the user clears all active sort keys and no filters remain
- **THEN** rows return to base source order without reopening the source

#### Scenario: One operation remains active
- **WHEN** the user clears one sort or filter while other clauses remain active
- **THEN** the remaining complete query is executed and atomically replaces the previous logical result

#### Scenario: Local query construction fails
- **WHEN** indexing, materialization, or local execution fails before a replacement result is complete
- **THEN** the base store and previously successful logical result remain unchanged

### Requirement: Query transitions preserve row identity
The viewer SHALL track cursor selection and marks by generation-scoped row and column identity across successful query transitions where those identities remain applicable.

#### Scenario: Selected row moves after sorting
- **WHEN** a successful sort moves the currently selected row to another result position
- **THEN** the cursor follows that row identity and retains the selected column identity when visible

#### Scenario: Selected row is filtered out
- **WHEN** a successful filter excludes the currently selected row
- **THEN** the viewer clamps the previous visible position into the new result and selects the applicable column

#### Scenario: Marked row is temporarily filtered out
- **WHEN** a query excludes a marked row
- **THEN** the mark retains its row and column identities and becomes reachable again if a later query includes that row in the same source generation

#### Scenario: Generation changes
- **WHEN** reload opens a new source generation
- **THEN** old row identities and marks are invalidated rather than applied to same-position rows in the new generation

### Requirement: Operation categories remain distinct
The system SHALL limit `TableQuery` to persistent row membership and row ordering while progressive navigation and scan/reduction operations use separate store interfaces.

#### Scenario: Search and skip remain progressive
- **WHEN** search or skip-to-change traverses the active result
- **THEN** it uses bounded row scans without adding a persistent filter or sort clause to `TableQuery`

#### Scenario: Column analysis is a reduction
- **WHEN** width calculation, numeric/type profiling, gradient range analysis, or identifier analysis inspects many rows
- **THEN** it uses sampled or exact scan/fold behavior without replacing the active result row set

#### Scenario: Exact reduction over an incremental store
- **WHEN** a reduction promises an exact full-dataset result
- **THEN** it scans through the required result under progress reporting without requiring a cloned row table solely for aggregation
