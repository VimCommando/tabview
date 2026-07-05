## Purpose

Define table model behavior and user-facing table operations for the Rust `tabview` viewer.

## Requirements

### Requirement: Header row behavior
The system SHALL classify and toggle the header row compatibly with the current viewer, while preserving accepted bug fixes from the Rust rewrite.

#### Scenario: Non-numeric first row
- **WHEN** a multi-row table has a first row with no numeric cells
- **THEN** the first row is treated as the header by default

#### Scenario: Toggle header
- **WHEN** a user presses `t`
- **THEN** the fixed header row is toggled on or off while preserving the selected data cell where possible

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
