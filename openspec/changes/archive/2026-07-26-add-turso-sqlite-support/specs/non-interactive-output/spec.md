## MODIFIED Requirements

### Requirement: Complete configured logical result
Batch output SHALL render the complete logical view after applying source options, the active bounded source result, and selected view configuration, including labels, column visibility and order, formats, widths, alignment, header visibility, view filters, view sort, null placement, and source-derived schema updates. Completion SHALL mean the entire active source result, not rows outside its configured source-query limit. Cursor position, viewport origin, selection styling, search state, and TUI-only start position SHALL NOT limit or decorate output.

#### Scenario: Saved view controls output
- **WHEN** an automatically selected or explicitly named saved view configures source operations, hides columns, formats values, filters rows, and sorts the view
- **THEN** batch output contains every row and visible column in the final view of the bounded source result

#### Scenario: SQLite source limit bounds output
- **WHEN** SQLite batch output uses a source limit of 1000
- **THEN** completion traverses at most those 1000 source rows even when the selected table contains more rows

#### Scenario: View filter does not refill output
- **WHEN** a view filter leaves 17 rows from a limited 1000-row SQLite result
- **THEN** output contains those 17 rows and does not query for replacements

#### Scenario: No saved view uses defaults
- **WHEN** no saved view applies
- **THEN** batch output uses source-defined headers, visible columns, display formatting, width mode, alignment defaults, source order, and the source's default result limit

#### Scenario: Start position does not truncate output
- **WHEN** a batch invocation includes an existing start-position argument
- **THEN** the complete bounded logical result is emitted because start position is an interactive cursor setting

#### Scenario: Late schema is included
- **WHEN** an incremental source discovers additional columns while completing its active result
- **THEN** applicable saved configuration is resolved before final output layout

### Requirement: Stable complete-table widths
Before writing the first table line, table output SHALL complete the active bounded source result and resolve one stable display width per visible column. It SHALL NOT cross a source-query limit to discover wider values. Explicit per-column widths SHALL be honored; otherwise each column SHALL expand to the widest normalized header or rendered value in that active result.

#### Scenario: Later wide value affects initial lines
- **WHEN** a value near the end of the active result is wider than earlier values
- **THEN** the header and preceding rows use that final wider column width

#### Scenario: Wider value lies beyond SQLite limit
- **WHEN** a wider database value exists outside the active source result
- **THEN** it does not affect output width and is not fetched for profiling

#### Scenario: Explicit width clips values
- **WHEN** saved view or CLI width configuration is smaller than a rendered value
- **THEN** that cell is clipped without shifting later columns

#### Scenario: Incremental result is fully traversed
- **WHEN** an incremental SQLite store supplies table output
- **THEN** Tabview traverses the complete bounded result for rows and width profiling without relying on a terminal viewport

### Requirement: Supported-source conversion
Every output adapter SHALL consume every compatible source format, including SQLite, through the shared table/view model rather than implementing source-specific exporters.

#### Scenario: CSV to text table
- **WHEN** a delimited input is rendered in table mode
- **THEN** its source-defined columns and rows are emitted as fixed-width text

#### Scenario: JSON to text table
- **WHEN** a JSON array or keyed JSON object is rendered in table mode
- **THEN** its resolved rows and columns use the same interpretation and saved-view rules as the TUI

#### Scenario: SQLite to text table
- **WHEN** a SQLite table is resolved and rendered in table mode
- **THEN** the existing output adapter serializes its bounded transformed view without SQLite-specific formatting code

## ADDED Requirements

### Requirement: Non-interactive SQLite table selection
Direct batch execution SHALL auto-select a sole selectable SQLite ordinary table or compatible ordinary view and SHALL require explicit CLI or saved selection when multiple selectable candidates remain.

#### Scenario: Sole table in batch mode
- **WHEN** direct batch execution discovers exactly one selectable candidate and no table is requested
- **THEN** it opens that candidate and emits its bounded view

#### Scenario: Saved table in batch mode
- **WHEN** direct batch execution discovers multiple selectable candidates and `source.table` resolves one
- **THEN** it opens that table without waiting for input

#### Scenario: Ambiguous batch database
- **WHEN** direct batch execution discovers multiple selectable candidates without `--table` or saved `source.table`
- **THEN** it writes no stdout, reports the candidates and required selection on stderr, and exits nonzero

#### Scenario: Interactive export can select
- **WHEN** `--interactive --output table` opens an ambiguous database
- **THEN** the startup table picker resolves the table before interaction and final export uses that selected table

### Requirement: SQLite output query completion
Direct and post-interactive output SHALL prepare the latest requested bounded SQLite source result before passing an immutable projection to the selected output adapter.

#### Scenario: Direct output waits for initial query
- **WHEN** the initial SQLite source query is still running
- **THEN** the output driver writes no stdout until the query and required bounded traversal succeed

#### Scenario: Interactive export waits for latest query
- **WHEN** normal interactive quit requests final output while a newer source-query revision is pending
- **THEN** final preparation awaits the latest revision before freezing and serializing the view

#### Scenario: Query preparation fails
- **WHEN** the required SQLite query or bounded traversal fails
- **THEN** no table bytes are emitted and the existing output error contract applies

#### Scenario: Generated SQL stays out of adapter output
- **WHEN** SQLite query provenance exists during normal table serialization
- **THEN** stdout contains only the selected output adapter's bytes
