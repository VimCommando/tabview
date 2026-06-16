## ADDED Requirements

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
The system SHALL support ascending and descending lexical, natural, and numeric sort on the current column using the existing keybindings.

#### Scenario: Numeric ascending sort
- **WHEN** a user presses `#`
- **THEN** rows are sorted by the current column using numeric comparison where values parse as numbers

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
