## ADDED Requirements

### Requirement: Store-backed table view
The viewer SHALL route interactive row access through a table-store abstraction rather than requiring the full dataset to be represented as `Vec<Vec<String>>` before rendering.

#### Scenario: Small file uses in-memory store
- **WHEN** a user opens a file below the lazy threshold
- **THEN** the viewer may parse the complete file into an in-memory store

#### Scenario: Large file uses incremental store
- **WHEN** a user opens a seekable file at or above the lazy threshold
- **THEN** the viewer uses an incremental file-backed store for initial rendering

### Requirement: Bounded initial load
The viewer SHALL render the first screen for a large seekable file after bounded initial sampling and parsing rather than full-file materialization.

#### Scenario: Initial viewport renders before full parse
- **WHEN** a large seekable file is opened
- **THEN** the viewer parses enough data for dialect selection, header classification, column metadata, initial widths, and visible rows
- **AND** it does not require every row to be parsed before the first frame

#### Scenario: Threshold remains centralized
- **WHEN** the system selects between in-memory and incremental stores
- **THEN** it uses the centralized 100 MiB default threshold unless future configuration overrides it

### Requirement: Incremental row indexing
The file-backed store SHALL index logical rows incrementally as navigation, search, or rendering needs rows beyond the currently indexed range.

#### Scenario: Navigation indexes forward
- **WHEN** the cursor moves beyond the currently indexed rows
- **THEN** the store indexes additional rows through the target row or until EOF

#### Scenario: Row count can be partial
- **WHEN** the large-file store has not reached EOF
- **THEN** the viewer can represent the row count as unknown or at-least-known without breaking navigation or rendering

### Requirement: Lazy-aware operations
The viewer SHALL define explicit behavior for operations over partially indexed data.

#### Scenario: Viewport-local operation
- **WHEN** a user opens a cell popup or yanks the current cell from an indexed row
- **THEN** the operation completes without materializing the full table

#### Scenario: Incremental search
- **WHEN** a search needs to inspect rows beyond the indexed range
- **THEN** the store indexes rows progressively until a match is found or EOF is reached

#### Scenario: Full-table sort
- **WHEN** a user invokes sort on a large file store
- **THEN** the system materializes or fully indexes the required rows in a controlled operation
- **AND** failure leaves the previous table state intact

### Requirement: Large-file status reporting
The TUI SHALL provide non-fatal status feedback for long-running indexing or materialization work.

#### Scenario: Materialization status
- **WHEN** a full-table operation requires materializing a large file
- **THEN** the viewer displays status indicating that indexing or materialization is in progress

#### Scenario: Large-file error
- **WHEN** incremental indexing or materialization fails
- **THEN** the viewer reports the failure non-fatally and preserves the last valid table state

### Requirement: Store-backed reload
Reload SHALL preserve view state where possible while rebuilding the appropriate in-memory or incremental store for the current input.

#### Scenario: Reload large file
- **WHEN** a user reloads a large file
- **THEN** cursor position, viewport origin, column width mode, column gap, per-column widths, and active search string are reapplied where possible after the store is rebuilt
