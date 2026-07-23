## ADDED Requirements

### Requirement: Store-backed table view
The viewer SHALL route interactive row access through a table-store abstraction rather than requiring the full dataset to be represented as `Vec<Vec<String>>` before rendering.

#### Scenario: Small input uses in-memory store
- **WHEN** an input can be safely materialized under the selected format's storage policy
- **THEN** the viewer may parse the complete table into an in-memory store

#### Scenario: Large seekable input uses incremental store
- **WHEN** a seekable delimited, JSON, or NDJSON input is at or above the centralized lazy threshold
- **THEN** the selected format adapter opens an incremental store suitable for initial rendering

#### Scenario: Storage strategy is format owned
- **WHEN** a format adapter opens a source for which filesystem size does not determine the appropriate storage strategy
- **THEN** the adapter can select its store without applying the delimited-file threshold mechanically

### Requirement: Bounded initial load
The viewer SHALL render the first screen for a large seekable input after format-appropriate bounded discovery and parsing rather than full-table materialization, except when the user explicitly requests a full schema scan.

#### Scenario: Initial viewport renders before full materialization
- **WHEN** a large seekable input is opened with the default schema scan policy
- **THEN** the adapter performs enough bounded work to define the initial columns, profile initial values, and provide visible rows
- **AND** it does not materialize every row before the first frame

#### Scenario: First frame fills the terminal viewport
- **WHEN** the first terminal layout has room for more rows than the provisional viewport used while opening the source
- **THEN** rendering indexes through the final visible row before drawing the first table frame
- **AND** the user does not need to scroll before the available row area is filled

#### Scenario: Sampled widths fit observed values
- **WHEN** default sampled width calculation observes values of different rendered widths in any supported source format
- **THEN** each automatic column width is at least the widest value observed in that sample
- **AND** indexing or scrolling beyond the initial sample does not change an existing automatic width

#### Scenario: Automatic widths respect the viewport
- **WHEN** an automatically calculated column width exceeds 80 percent of the terminal viewport width
- **THEN** the automatic width is capped at 80 percent of the viewport
- **AND** an explicit user width or subsequent manual growth may exceed that automatic cap

#### Scenario: Single-column resizing scales within cached values
- **WHEN** the user shrinks or grows the current column with `,` or `.`
- **THEN** each step changes its current width by 20 percent with a minimum one-character adjustment
- **AND** shrinking stops at one character while growth stops at the widest currently cached rendered value in that column

#### Scenario: Lazy threshold remains centralized
- **WHEN** a size-based adapter selects between in-memory and incremental stores
- **THEN** it uses the centralized 100 MiB default lazy threshold unless configuration overrides it

#### Scenario: Full schema scan is explicit
- **WHEN** a user requests a full schema scan for a large structured input
- **THEN** the viewer reports schema scanning status and may delay its first frame until the scan reaches the selected table's end

### Requirement: Incremental logical-row indexing
An incremental store SHALL index logical rows as navigation, search, skip, or rendering needs rows beyond the currently indexed range.

#### Scenario: Navigation indexes forward
- **WHEN** the cursor moves beyond the currently indexed rows
- **THEN** the store indexes additional logical rows through the target row or until the selected table ends

#### Scenario: Jump to end scans sequentially
- **WHEN** a user presses `G` on a large incremental file
- **THEN** the viewer indexes and loads the remaining logical rows in one forward parser pass that records logical offsets and retains each decoded row in the active view cache
- **AND** it does not parse the same remaining range once for indexing and again for row loading
- **AND** it does not recompute frozen sampled widths or profiles for every newly indexed row

#### Scenario: Multi-line delimited record
- **WHEN** a quoted delimited record spans multiple physical lines
- **THEN** the store indexes it as one logical row rather than treating each newline as a row boundary

#### Scenario: Row count can be partial
- **WHEN** an incremental store has not reached the selected table's end
- **THEN** the viewer represents the row count as unknown or at-least-known without breaking navigation or rendering

### Requirement: Lazy-aware operations
The viewer SHALL give every existing table operation explicit behavior over a partially indexed store and SHALL NOT materialize the full table for a viewport-local operation.

#### Scenario: Viewport-local operation
- **WHEN** a user renders indexed rows, opens the current cell, views table information, or yanks the current cell
- **THEN** the operation accesses only required indexed rows and does not clone or materialize the full visible table

#### Scenario: Final column is partially visible
- **WHEN** the next visible column begins within the terminal width but its configured width extends beyond the right edge
- **THEN** the viewer renders and clips that column into the remaining screen cells instead of omitting it entirely
- **AND** the same clipping boundary is used for its header and data cells

#### Scenario: Progressive search or skip
- **WHEN** search or skip-to-change needs to inspect rows beyond the indexed range
- **THEN** the store indexes and scans rows progressively until a result is found or the selected table ends

#### Scenario: Local full-table sort or filter
- **WHEN** the generic local executor performs sort, filter, or another exact full-table operation
- **THEN** the system fully indexes, scans, or materializes the required data in a controlled operation
- **AND** failure leaves the prior table and view state intact

#### Scenario: Source-executed query remains lazy
- **WHEN** a store executes a complete sort/filter query with exact semantics
- **THEN** its derived result may expose unknown or at-least-known row count and fetch result rows incrementally without local full-table materialization

#### Scenario: Exact full-table profiling
- **WHEN** max-width sizing, auto-range color profiling, or another operation promises an exact full-dataset result
- **THEN** the system performs controlled full indexing or clearly uses a documented sampled result

### Requirement: Large-operation status reporting
The TUI SHALL provide non-fatal status feedback for schema scanning, indexing, and materialization work that can delay interaction.

#### Scenario: Initial loading status
- **WHEN** the terminal session begins opening an input and no table-data frame has rendered
- **THEN** the status bar displays `Loading <filename>` using the input's user-facing display name
- **AND** the loading message remains visible until the first table-data frame renders successfully

#### Scenario: Long-running operation status
- **WHEN** an operation performs a full schema scan, indexes a substantial new range, or materializes a store
- **THEN** the viewer displays status identifying the work in progress

#### Scenario: Initial open fails
- **WHEN** source opening fails before the first table-data frame renders
- **THEN** the system restores terminal state before reporting the opening error

#### Scenario: Incremental store error
- **WHEN** schema discovery, indexing, row decoding, or materialization fails
- **THEN** the viewer reports the failure non-fatally when possible and preserves the last valid table state

### Requirement: Store-backed reload
Reload SHALL reopen the source through the same format-aware path and preserve view state where possible.

#### Scenario: Reload incremental input
- **WHEN** a user reloads a large or structured input
- **THEN** the viewer reapplies the selected format, source options, cursor position, viewport origin, column width settings, active search, and applicable saved-view state where possible

#### Scenario: Reload discovers schema changes
- **WHEN** reloading a provisional or changed structured source produces a different schema
- **THEN** the viewer rebuilds column definitions and reapplies saved configuration by stable source identity where possible
