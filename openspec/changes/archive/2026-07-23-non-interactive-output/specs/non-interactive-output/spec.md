## ADDED Requirements

### Requirement: Output mode resolution
Tabview SHALL resolve the independent `--interactive` flag and optional `--output <format>` before entering a terminal session. With neither option, terminal stdout SHALL select a view-only TUI and non-terminal stdout SHALL select immediate `table` output. `--interactive` alone SHALL select a view-only TUI. `--output <format>` alone SHALL select that batch adapter. Their combination SHALL run the TUI and serialize its final live view through that adapter on normal quit.

#### Scenario: Terminal stdout remains automatically interactive
- **WHEN** neither `--interactive` nor `--output` is supplied and stdout is a terminal
- **THEN** Tabview enters the interactive TUI using existing behavior

#### Scenario: Redirected stdout selects table output
- **WHEN** neither `--interactive` nor `--output` is supplied and stdout is redirected to a file
- **THEN** Tabview writes a non-interactive table to stdout without entering the TUI

#### Scenario: Pipeline selects table output
- **WHEN** neither `--interactive` nor `--output` is supplied and stdout is connected to another process
- **THEN** Tabview writes a non-interactive table to the pipe

#### Scenario: Explicit table output to terminal
- **WHEN** `--output table` is supplied without `--interactive` and stdout is a terminal
- **THEN** Tabview writes the table once and exits without entering the TUI

#### Scenario: Interactive transform with redirected stdout
- **WHEN** `--interactive --output table` is supplied, stdout is not a terminal, and a controlling terminal is available
- **THEN** Tabview runs the UI on the controlling terminal and reserves stdout for final table serialization

#### Scenario: Interactive mode without a controlling terminal
- **WHEN** `--interactive` is supplied, stdin/stdout are data streams, and no controlling terminal is available
- **THEN** Tabview fails before consuming input or entering raw mode, writes a clear diagnostic to stderr, and emits no stdout

### Requirement: Explicit interactive transformation
Combining `--interactive` with `--output <format>` SHALL treat the TUI as an interactive transformation stage. On normal quit, Tabview SHALL restore the terminal, complete input ingestion and late schema resolution, freeze the final live view state, prepare the complete logical result, and serialize it through the selected output adapter to stdout. Interactive sessions without `--output` SHALL NOT serialize their final live state.

#### Scenario: Live modifications control final output
- **WHEN** a user combines `--interactive` with an output format, then hides columns, changes formats, filters rows, or changes sort order before normal quit
- **THEN** stdout contains the complete final logical result with those live modifications applied

#### Scenario: Screen state is excluded
- **WHEN** an interactive transform has cursor, viewport, selection, popup, or search-highlight state at normal quit
- **THEN** those screen-only details do not restrict or decorate the serialized result

#### Scenario: Interactive mode without output does not export
- **WHEN** automatic mode or explicit `--interactive` selects the TUI without `--output` and the user quits normally
- **THEN** Tabview restores the terminal and exits without serializing the final live view to stdout

#### Scenario: Cancellation or failure does not export
- **WHEN** an interactive transform is cancelled or fails during terminal use, ingestion, final preparation, or terminal restoration
- **THEN** Tabview emits no final table and exits according to the failure or cancellation contract

### Requirement: Terminal and data channel separation
When interactive input or output occupies standard streams, Tabview SHALL use an available controlling terminal for UI events and drawing while reserving stdin for source bytes and stdout for serialized result bytes. UI control sequences, loading indicators, and screen content SHALL NOT be written to redirected stdout.

#### Scenario: Provisional schema from piped stdin
- **WHEN** interactive mode receives a non-seekable stdin source
- **THEN** Tabview buffers enough input to establish a provisional schema and display the table, then continues draining and materializing input while interaction proceeds

#### Scenario: Quit completes finite input
- **WHEN** the user normally quits an interactive transform before a finite stdin producer reaches EOF
- **THEN** Tabview completes ingestion and late-schema application before preparing and writing the final result

#### Scenario: Terminal restored before output
- **WHEN** an interactive transform quits normally
- **THEN** raw mode and alternate-screen state are restored before the output adapter writes any final bytes

### Requirement: Non-interactive execution path
In any batch output format, Tabview SHALL NOT enable raw mode, enter the alternate screen, draw loading/footer chrome, read terminal events, access the clipboard, or wait for user input.

#### Scenario: Table mode has no terminal side effects
- **WHEN** table output is selected
- **THEN** source opening, view application, rendering, and process exit occur without constructing a terminal session

#### Scenario: Piped stdin and stdout
- **WHEN** input is read from stdin and table output is piped to another process
- **THEN** Tabview consumes stdin as data, writes the formatted table to stdout, and never attempts to read interactive input

### Requirement: Complete configured logical result
Table mode SHALL render the complete logical result after applying source options and selected view configuration, including labels, column visibility and order, formats, widths, alignment, header visibility, filters, sort order, null placement, and source-derived schema updates. Cursor position, viewport origin, selection styling, search state, and TUI-only start position SHALL NOT limit or decorate the output.

#### Scenario: Saved view controls output
- **WHEN** an automatically selected or explicitly named saved view hides columns, formats values, filters rows, and sorts the result
- **THEN** table output contains every row and visible column in that configured logical result and no hidden columns

#### Scenario: No saved view uses defaults
- **WHEN** no saved view applies
- **THEN** table output uses the normal source-defined headers, visible columns, display formatting, width mode, alignment defaults, and source order

#### Scenario: Start position does not truncate output
- **WHEN** a table-mode invocation includes an existing start-position argument
- **THEN** the complete logical result is emitted because start position is an interactive cursor setting

#### Scenario: Late schema is included
- **WHEN** a structured incremental source discovers additional columns while completing the output
- **THEN** applicable saved configuration is resolved for those columns and the final output layout includes every resulting visible column

### Requirement: Stable complete-table widths
Before writing the first table line, table mode SHALL complete the required source/query traversal and resolve one stable display width per visible column. It SHALL NOT derive an aggregate width from terminal dimensions, impose a total row-width cap, or automatically wrap or reflow output. Explicitly configured per-column fixed or maximum widths SHALL be honored; without an explicit per-column cap, each column SHALL expand to the widest normalized header or rendered value in the complete logical result.

#### Scenario: Later wide value affects initial lines
- **WHEN** a value near the end of the result is wider than earlier values and its column uses automatic width
- **THEN** the header and every preceding row are padded using the final wider column width

#### Scenario: Explicit width clips values
- **WHEN** a saved view or CLI width mode supplies an explicit column width smaller than a rendered value
- **THEN** that cell is clipped to the configured display width without shifting subsequent columns

#### Scenario: Wide output remains unconstrained
- **WHEN** the complete result requires a row wider than the terminal or downstream viewport and no per-column cap is configured
- **THEN** Tabview emits the full-width row without wrapping, reflowing, or clipping it to an aggregate limit

#### Scenario: Consumer controls presentation width
- **WHEN** a caller wants truncation, wrapping, paging, horizontal scrolling, or reflow
- **THEN** the caller composes table output with an appropriate downstream consumer rather than relying on terminal-width detection in Tabview

#### Scenario: Incremental input is fully traversed
- **WHEN** an incremental store supplies table output
- **THEN** Tabview performs controlled complete traversal for rows, pending schema, query semantics, and width profiling before emission without relying on a terminal viewport

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
Tabview SHALL resolve color mode as `auto`, `always`, or `never`. In table output, `auto` and `never` SHALL emit no ANSI control sequences, while `always` SHALL emit ANSI styles derived from the resolved theme for headers, ordinary cells, and configured conditional cell colors.

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
- **THEN** Tabview stops writing and exits cleanly without printing a broken-pipe error

#### Scenario: Other write failure
- **WHEN** stdout writing fails for a reason other than `BrokenPipe`
- **THEN** Tabview reports the failure on stderr and exits nonzero

#### Scenario: Same-file shell redirection is not supported
- **WHEN** a caller redirects final output to the same pathname used as input
- **THEN** safe in-place replacement is outside Tabview's contract because the shell may truncate the file before process startup; documentation directs callers to a distinct destination and notes that fixed-width table output does not preserve CSV or JSON source format

### Requirement: Modular output adapters
Tabview SHALL dispatch each selected `OutputFormat` through a source-neutral output adapter in both direct and post-interactive lifecycles. Shared orchestration SHALL open the source, apply the saved or frozen live view, satisfy the adapter's declared preparation requirements, validate adapter capabilities, provide an immutable prepared projection, and own stdout, stderr, broken-pipe, and exit-status behavior. Format-specific adapters SHALL own only their layout, escaping, styling, and byte serialization rules.

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
- **THEN** Tabview rejects the invocation before writing stdout with a clear diagnostic on stderr

### Requirement: Supported-source conversion
Every output adapter SHALL consume every compatible source format supported by the normal format-aware opening path through the source-neutral table/view model rather than implementing input-format-specific exporters.

#### Scenario: CSV to text table
- **WHEN** a delimited input is piped or explicitly rendered in table mode
- **THEN** its source-defined columns and rows are emitted as fixed-width text

#### Scenario: JSON to text table
- **WHEN** a JSON array or keyed JSON object is rendered in table mode
- **THEN** its resolved table rows and columns are emitted using the same JSON interpretation and saved-view rules as the interactive viewer

#### Scenario: Future source adapter
- **WHEN** a future adapter supplies a valid opened table and store
- **THEN** each compatible output adapter can render it without source-specific output code
