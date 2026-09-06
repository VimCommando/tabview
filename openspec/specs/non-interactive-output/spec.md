## Purpose

Define runtime/output-format selection, interactive transformation export, complete fixed-width table rendering, color policy, stream/error behavior, and source-neutral output adapter semantics.

## Requirements

### Requirement: Output mode resolution
Tview SHALL resolve the independent `--interactive` flag and optional `--output <format>` before entering a terminal session. With neither option, terminal stdout SHALL select a view-only TUI and non-terminal stdout SHALL select immediate `table` output. `--interactive` alone SHALL select a view-only TUI. `--output <format>` alone SHALL select that batch adapter. Their combination SHALL run the TUI and serialize its final live view through that adapter on normal quit.

#### Scenario: Terminal stdout remains automatically interactive
- **WHEN** neither `--interactive` nor `--output` is supplied and stdout is a terminal
- **THEN** Tview enters the interactive TUI using existing behavior

#### Scenario: Redirected stdout selects table output
- **WHEN** neither `--interactive` nor `--output` is supplied and stdout is redirected to a file
- **THEN** Tview writes a non-interactive table to stdout without entering the TUI

#### Scenario: Pipeline selects table output
- **WHEN** neither `--interactive` nor `--output` is supplied and stdout is connected to another process
- **THEN** Tview writes a non-interactive table to the pipe

#### Scenario: Explicit table output to terminal
- **WHEN** `--output table` is supplied without `--interactive` and stdout is a terminal
- **THEN** Tview writes the table once and exits without entering the TUI

#### Scenario: Interactive transform with redirected stdout
- **WHEN** `--interactive --output table` is supplied, stdout is not a terminal, and a controlling terminal is available
- **THEN** Tview runs the UI on the controlling terminal and reserves stdout for final table serialization

#### Scenario: Interactive mode without a controlling terminal
- **WHEN** `--interactive` is supplied, stdin/stdout are data streams, and no controlling terminal is available
- **THEN** Tview fails before consuming input or entering raw mode, writes a clear diagnostic to stderr, and emits no stdout
### Requirement: Explicit interactive transformation
Combining `--interactive` with `--output <format>` SHALL treat the TUI as an interactive transformation stage. On normal quit, Tview SHALL restore the terminal, complete input ingestion and late schema resolution, freeze the final live view state, prepare the complete logical result, and serialize it through the selected output adapter to stdout. Interactive sessions without `--output` SHALL NOT serialize their final live state.

#### Scenario: Live modifications control final output
- **WHEN** a user combines `--interactive` with an output format, then hides columns, changes formats, filters rows, or changes sort order before normal quit
- **THEN** stdout contains the complete final logical result with those live modifications applied

#### Scenario: Screen state is excluded
- **WHEN** an interactive transform has cursor, viewport, selection, popup, or search-highlight state at normal quit
- **THEN** those screen-only details do not restrict or decorate the serialized result

#### Scenario: Interactive mode without output does not export
- **WHEN** automatic mode or explicit `--interactive` selects the TUI without `--output` and the user quits normally
- **THEN** Tview restores the terminal and exits without serializing the final live view to stdout

#### Scenario: Cancellation or failure does not export
- **WHEN** an interactive transform is cancelled or fails during terminal use, ingestion, final preparation, or terminal restoration
- **THEN** Tview emits no final table and exits according to the failure or cancellation contract
### Requirement: Terminal and data channel separation
When interactive input or output occupies standard streams, Tview SHALL use an available controlling terminal for UI events and drawing while reserving stdin for source bytes and stdout for serialized result bytes. UI control sequences, loading indicators, and screen content SHALL NOT be written to redirected stdout.

#### Scenario: Provisional schema from piped stdin
- **WHEN** interactive mode receives a non-seekable stdin source
- **THEN** Tview buffers enough input to establish a provisional schema and display the table, then continues draining and materializing input while interaction proceeds

#### Scenario: Quit completes finite input
- **WHEN** the user normally quits an interactive transform before a finite stdin producer reaches EOF
- **THEN** Tview completes ingestion and late-schema application before preparing and writing the final result

#### Scenario: Terminal restored before output
- **WHEN** an interactive transform quits normally
- **THEN** raw mode and alternate-screen state are restored before the output adapter writes any final bytes
### Requirement: Non-interactive execution path
In any batch output format, Tview SHALL NOT enable raw mode, enter the alternate screen, draw loading/footer chrome, read terminal events, access the clipboard, or wait for user input.

#### Scenario: Table mode has no terminal side effects
- **WHEN** table output is selected
- **THEN** source opening, view application, rendering, and process exit occur without constructing a terminal session

#### Scenario: Piped stdin and stdout
- **WHEN** input is read from stdin and table output is piped to another process
- **THEN** Tview consumes stdin as data, writes the formatted table to stdout, and never attempts to read interactive input
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
- **THEN** Tview traverses the complete bounded result for rows and width profiling without relying on a terminal viewport
### Requirement: Deterministic fixed-width text format
Plain table output SHALL emit zero or more newline-terminated physical lines. Each included header or data row SHALL contain visible cells in configured order, aligned and clipped by Unicode display width, separated by exactly the configured column gap, with no leading location field, borders, divider line, hidden-column markers, footer, or trailing spaces after the final cell.

#### Scenario: Header and rows
- **WHEN** header visibility is enabled for a table containing data
- **THEN** the first output line is the formatted header and each subsequent line is one formatted logical row

#### Scenario: Header is hidden
- **WHEN** header visibility is disabled
- **THEN** output begins with the first data row and contains no replacement heading or divider

#### Scenario: Empty result with header
- **WHEN** the logical result has zero rows but has visible columns and header visibility is enabled
- **THEN** output contains only the formatted header line

#### Scenario: Empty result without header
- **WHEN** the logical result has zero rows and header visibility is disabled
- **THEN** stdout receives zero bytes

#### Scenario: Right-aligned numeric cell
- **WHEN** a visible column resolves to right alignment
- **THEN** each shorter cell is left-padded to its resolved display width

#### Scenario: Left-aligned text cell
- **WHEN** a visible column resolves to left alignment and is followed by another column
- **THEN** each shorter cell is right-padded to its resolved display width before the column gap

#### Scenario: Unicode width and clipping
- **WHEN** a cell contains wide or combining Unicode characters
- **THEN** padding and clipping use terminal display width, preserve valid UTF-8, and do not exceed the resolved column width

#### Scenario: Embedded control characters
- **WHEN** a rendered cell contains newline, carriage-return, tab, escape, or another control character
- **THEN** table mode replaces it with a visible escaped representation so one logical row remains one physical output line
### Requirement: Non-interactive color policy
Tview SHALL resolve color mode as `auto`, `always`, or `never`. In table output, `auto` and `never` SHALL emit no ANSI control sequences, while `always` SHALL emit ANSI styles derived from the resolved theme for headers, ordinary cells, and configured conditional cell colors.

#### Scenario: Piped output is plain by default
- **WHEN** table output uses default `auto` color mode
- **THEN** stdout contains no ANSI escape sequences even if a theme defines colors

#### Scenario: Color is explicitly enabled
- **WHEN** table output uses color mode `always`
- **THEN** emitted header and cell content uses theme-derived ANSI styling and resets styles before unstyled separators or line termination

#### Scenario: Color is explicitly disabled
- **WHEN** color mode is `never`
- **THEN** no ANSI color or modifier sequence is written in either automatic or explicitly selected table output

#### Scenario: Styling does not affect width
- **WHEN** ANSI styling is enabled
- **THEN** escape sequences do not contribute to clipping, alignment, or padding calculations
### Requirement: Clean stdout and stderr contract
Batch output and interactive final export SHALL reserve stdout for adapter bytes, write warnings and errors to stderr, return a nonzero status for failures other than downstream pipe closure, and treat `BrokenPipe` while writing stdout as a clean early termination without an additional diagnostic.

#### Scenario: Saved-view warning does not corrupt table
- **WHEN** saved-view or theme resolution produces a warning in table mode
- **THEN** the warning is written to stderr and stdout contains only table output

#### Scenario: Opening fails before output
- **WHEN** source opening, view application, full traversal, or width profiling fails before the first line is written
- **THEN** stdout remains empty, stderr describes the failure, and the process exits nonzero

#### Scenario: Downstream consumer exits early
- **WHEN** a command such as `head` closes the stdout pipe before all rows are written
- **THEN** Tview stops writing and exits cleanly without printing a broken-pipe error

#### Scenario: Other write failure
- **WHEN** stdout writing fails for a reason other than `BrokenPipe`
- **THEN** Tview reports the failure on stderr and exits nonzero

#### Scenario: Same-file shell redirection is not supported
- **WHEN** a caller redirects final output to the same pathname used as input
- **THEN** safe in-place replacement is outside Tview's contract because the shell may truncate the file before process startup; documentation directs callers to a distinct destination and notes that fixed-width table output does not preserve CSV or JSON source format
### Requirement: Modular output adapters
Tview SHALL dispatch each selected `OutputFormat` through a source-neutral output adapter in both direct and post-interactive lifecycles. Shared orchestration SHALL open the source, apply the saved or frozen live view, satisfy the adapter's declared preparation requirements, validate adapter capabilities, provide an immutable prepared projection, and own stdout, stderr, broken-pipe, and exit-status behavior. Format-specific adapters SHALL own only their layout, escaping, styling, and byte serialization rules.

#### Scenario: Fixed-width table adapter
- **WHEN** resolved output format is `table`
- **THEN** the batch driver selects the fixed-width adapter and supplies its requested complete projection and width/style preparation

#### Scenario: Interactive mode reuses selected adapter
- **WHEN** interactive mode quits normally with `--output table`
- **THEN** the shared output driver selects the same fixed-width table adapter using the frozen live view rather than a separate TUI exporter

#### Scenario: Future Markdown adapter
- **WHEN** a future `markdown` output value and adapter are added
- **THEN** it can reuse source opening, saved-view application, prepared projection, diagnostics, and stream handling while defining Markdown-specific escaping and layout without changing the TUI or table adapter

#### Scenario: Unsupported adapter capability
- **WHEN** an output option such as `--color always` is incompatible with the selected adapter
- **THEN** Tview rejects the invocation before writing stdout with a clear diagnostic on stderr
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
