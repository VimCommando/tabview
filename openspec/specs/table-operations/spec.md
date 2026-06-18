## Purpose

Define table model behavior and user-facing table operations for the Rust `tabview` viewer.

## Requirements

### Requirement: Header row behavior
The system SHALL classify and toggle the header row compatibly with the current viewer, while allowing compatibility tests to mark selected quirks as accepted bug fixes.

#### Scenario: Non-numeric first row
- **WHEN** a multi-row table has a first row with no numeric cells
- **THEN** the first row is treated as the header by default

#### Scenario: Toggle header
- **WHEN** a user presses `t`
- **THEN** the fixed header row is toggled on or off while preserving the selected data cell where possible

### Requirement: Column sizing controls
The system SHALL support fixed, mode, and max column width modes plus interactive width and gap adjustments using the existing keys.

#### Scenario: Increase current column width
- **WHEN** a user presses `.`
- **THEN** the current column width increases and the viewport layout is recalculated

#### Scenario: Set fixed width with modifier
- **WHEN** a user presses `20c`
- **THEN** all columns use fixed width 20 subject to terminal constraints

### Requirement: Sort operations
The system SHALL support ascending and descending lexical, natural, and numeric sort on the current column using the existing keybindings. Numeric sort SHALL treat plain numbers, recognized suffixed numbers, and multi-dot numeric values as numeric values, while leaving non-numeric values after numeric values in ascending order.

#### Scenario: Numeric ascending sort
- **WHEN** a user presses `#`
- **THEN** rows are sorted by the current column using numeric comparison where values parse as numbers

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

### Requirement: Search traversal
The system SHALL preserve current forward and reverse search traversal results, including wraparound through rows and columns, without mutating table row or cell order during traversal.

#### Scenario: Next search result
- **WHEN** a search string is active and the user presses `n`
- **THEN** the cursor moves to the next matching cell after the current cell, wrapping when needed

#### Scenario: Previous search result
- **WHEN** a search string is active and the user presses `p`
- **THEN** the cursor moves to the previous matching cell, wrapping when needed

#### Scenario: Reverse search preserves table order
- **WHEN** a user presses `p` to search backward
- **THEN** the table row order and cell order remain unchanged after the search completes

### Requirement: Skip-to-change operations
The system SHALL support skipping to the next or previous change in row or column value using `[`, `]`, `{`, and `}` with optional numeric modifiers.

#### Scenario: Skip to next row value change
- **WHEN** a user presses `]`
- **THEN** the cursor moves downward in the current column to the next row whose value differs from the starting cell

### Requirement: Clipboard operation
The system SHALL support yanking the current cell contents when compiled with clipboard support and SHALL fail non-fatally when clipboard support is disabled or unavailable.

#### Scenario: Clipboard enabled
- **WHEN** clipboard support is enabled and the user presses `y`
- **THEN** the current cell contents are copied to the system clipboard

#### Scenario: Clipboard unavailable
- **WHEN** clipboard support is disabled or unavailable and the user presses `y`
- **THEN** the viewer continues running without corrupting state
