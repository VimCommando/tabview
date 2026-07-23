## Purpose

Define table model behavior and user-facing table operations for the Rust `tabview` viewer.

## Requirements

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
- **THEN** the fixed header row is toggled on or off while preserving the selected data cell where possible

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

### Requirement: Column sizing controls
The system SHALL support fixed, mode, and max column width modes plus interactive width and gap adjustments using `z` and `Z` for the former all-column and current-column width commands.

#### Scenario: Increase current column width
- **WHEN** a user presses `.`
- **THEN** the current column width increases and the viewport layout is recalculated

#### Scenario: Set fixed width with modifier
- **WHEN** a user presses `20z`
- **THEN** all columns use fixed width 20 subject to terminal constraints

#### Scenario: Toggle all-column width mode
- **WHEN** a user presses `z` without a numeric prefix
- **THEN** the viewer toggles variable column width mode between `mode` and `max`

#### Scenario: Maximize current column
- **WHEN** a user presses `Z` without a numeric prefix
- **THEN** the current column width is maximized using existing max-content sizing behavior

#### Scenario: Set current column width with modifier
- **WHEN** a user presses `20Z`
- **THEN** the current column uses fixed width 20 subject to terminal constraints

### Requirement: Sort operations
The system SHALL support ascending and descending lexical, natural, numeric, and type-aware multi-level sort on the current column using the existing keybindings plus composable column sort commands. Numeric sort SHALL treat plain numbers, recognized suffixed numbers, and multi-dot numeric values as numeric values, while leaving non-numeric values after numeric values in ascending order. Shortcut sort operations SHALL maintain an ordered sort list with at most three entries.

#### Scenario: Numeric ascending sort
- **WHEN** a user presses `#`
- **THEN** rows are sorted by the current column using numeric comparison where values parse as numbers, and the current column becomes the primary sort key

#### Scenario: Scientific and byte suffix numeric sort
- **WHEN** numeric sort is applied to values with scientific suffixes from nano through exa, byte suffixes such as `kb`, `MB`, `GiB`, and `MiB`, or decimal percent suffixes such as `2.5%`
- **THEN** those values are compared using their numeric magnitude, with `%` using no multiplier beyond the numeric value itself

#### Scenario: Time-context suffix numeric sort
- **WHEN** a numeric column contains explicit time suffixes such as `ns`, `us`, `ms`, `s`, `min`, `h`, `d`, or `y`, or the column header suggests time-like data such as duration, latency, elapsed, runtime, uptime, timeout, or interval
- **THEN** numeric sort treats bare `m` as minutes for that column

#### Scenario: Non-time bare m numeric sort
- **WHEN** a numeric column does not have time-context evidence
- **THEN** numeric sort treats bare `m` as the scientific milli suffix

#### Scenario: Multi-dot numeric sort
- **WHEN** numeric sort is applied to values with multiple dot-separated numeric groups such as IP addresses or semantic versions
- **THEN** those values are compared component-by-component numerically

#### Scenario: Placeholder values in numeric columns
- **WHEN** a numeric column contains placeholder values such as `null`, `n/a`, `na`, `none`, `nil`, or `nan`
- **THEN** those placeholders do not prevent the column from being treated as numeric and sort after numeric values in ascending order

#### Scenario: Sticky numeric column profile
- **WHEN** the viewer classifies a column as time-context or default numeric context
- **THEN** that numeric interpretation remains stable for subsequent sorts and rendering until the table is reloaded or reclassified

#### Scenario: Numeric column alignment
- **WHEN** a visible column contains only numeric values, empty cells, or recognized placeholder values
- **THEN** data cells in that column are right-aligned while headers remain left-aligned

#### Scenario: Shortcut sort keeps last three keys
- **WHEN** a user sorts columns A, B, C, and D using `s/S`, `a/A`, or `#/@` shortcuts
- **THEN** the sort list keeps D as the primary key followed by C and B as trailing sort keys, and drops A

#### Scenario: Shortcut sort removes duplicate column
- **WHEN** a user sorts column A, then column B, then column A again using `s/S`, `a/A`, or `#/@` shortcuts
- **THEN** column A becomes the primary sort key and its previous trailing entry is removed

#### Scenario: Repeated shortcut toggles sort off
- **WHEN** a column is already sorted with the same kind and direction requested by `s`, `S`, `a`, `A`, `#`, or `@`
- **THEN** pressing that shortcut again removes that column from the sort list

#### Scenario: Column sort ascending command
- **WHEN** a user presses `csk`
- **THEN** the current column becomes the primary ascending sort key using numeric sort for number-family columns and lexical sort for all other columns

#### Scenario: Column sort descending command
- **WHEN** a user presses `csj`
- **THEN** the current column becomes the primary descending sort key using numeric sort for number-family columns and lexical sort for all other columns

#### Scenario: Column sort clear command
- **WHEN** a user presses `csx`
- **THEN** the current column is removed from the sort list without changing the remaining sort key order

#### Scenario: Sorted header indicators
- **WHEN** a visible column participates in the sort list
- **THEN** its header displays `▲` for ascending sort or `▼` for descending sort

### Requirement: Search traversal
The system SHALL preserve current forward and reverse search traversal results, including wraparound through rows and columns, without mutating table row or cell order during traversal.

#### Scenario: Next search result
- **WHEN** a search string is active and the user presses `n`
- **THEN** the cursor moves to the next matching cell after the current cell, wrapping when needed

#### Scenario: Previous search result
- **WHEN** a search string is active and the user presses `N`
- **THEN** the cursor moves to the previous matching cell, wrapping when needed

#### Scenario: Reverse search preserves table order
- **WHEN** a user presses `N` to search backward
- **THEN** the table row order and cell order remain unchanged after the search completes

### Requirement: Search match values
The system SHALL match search queries against both raw cell values and saved-view-rendered cell values.

#### Scenario: Search matches raw value
- **WHEN** a saved view renders raw value `1000` as `1,000` and the user searches for `1000`
- **THEN** the cell is included in search traversal results

#### Scenario: Search matches rendered value
- **WHEN** a saved view renders raw value `1000` as `1,000` and the user searches for `1,000`
- **THEN** the cell is included in search traversal results

#### Scenario: Search highlights matching cell
- **WHEN** a search query matches either the raw value or rendered value of a cell
- **THEN** search traversal highlights that cell regardless of which representation matched

### Requirement: Column visibility controls
The system SHALL support composable column show and hide commands under the `c` prefix, using `h` for hide, `H` for show, and directional suffixes.

#### Scenario: Hide current column
- **WHEN** a user presses `chj` or `chk`
- **THEN** the current column is hidden and the cursor moves to the nearest visible column when possible

#### Scenario: Hide columns to the right
- **WHEN** a user presses `10chl`
- **THEN** the system hides up to 10 visible columns to the right of the current column, nearest first

#### Scenario: Hide columns to the left
- **WHEN** a user presses `chh`
- **THEN** the system hides one visible column to the left of the current column when one exists

#### Scenario: Show hidden columns to the left
- **WHEN** a user presses `cHh`
- **THEN** the system shows the nearest hidden column immediately adjacent to the left of the current column in source order when one exists

#### Scenario: Show hidden columns to the right
- **WHEN** a user presses `5cHl`
- **THEN** the system shows up to 5 hidden columns immediately adjacent to the right of the current column in source order, nearest first

#### Scenario: Prevent hiding every column
- **WHEN** a column hide command would hide the last visible column
- **THEN** the viewer leaves at least one column visible and reports the condition through the footer message line

#### Scenario: Hidden column header indicator
- **WHEN** one or more hidden source columns exist between visible headers or beyond a visible edge
- **THEN** the header row displays a `|` indicator at that boundary

### Requirement: Sort persistence in saved views
The system SHALL include active sort state as an ordered list when serializing the current runtime view configuration to saved view YAML.

#### Scenario: Persist active sort
- **WHEN** a sort is active and the user opens the saved view modal
- **THEN** the generated YAML includes an ordered `sort` list containing each sort key's source column, direction, and kind

#### Scenario: Restore saved sort
- **WHEN** a saved view file contains a `sort` list whose source columns exist in the loaded table
- **THEN** the system applies the sort keys after loading the table and resolving columns, preserving list order as the multi-level sort precedence

#### Scenario: Search is not persisted
- **WHEN** a search query is active and the user opens the saved view modal
- **THEN** the generated YAML does not include the search query

### Requirement: Skip-to-change operations
The system SHALL support skipping to the next or previous change in row or column value using `[`, `]`, `{`, and `}` with optional numeric modifiers.

#### Scenario: Skip to next row value change
- **WHEN** a user presses `]`
- **THEN** the cursor moves downward in the current column to the next row whose value differs from the starting cell

### Requirement: Clipboard operation
The system SHALL support yanking the rendered current cell contents with `y` and the raw current cell contents with `Y` when compiled with clipboard support, and SHALL fail non-fatally when clipboard support is disabled or unavailable.

#### Scenario: Clipboard enabled rendered yank
- **WHEN** clipboard support is enabled and the user presses `y`
- **THEN** the rendered current cell contents are copied to the system clipboard

#### Scenario: Clipboard enabled raw yank
- **WHEN** clipboard support is enabled and the user presses `Y`
- **THEN** the raw current cell contents are copied to the system clipboard

#### Scenario: Clipboard unavailable
- **WHEN** clipboard support is disabled or unavailable and the user presses `y` or `Y`
- **THEN** the viewer continues running without corrupting state
