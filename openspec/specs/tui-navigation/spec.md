## Purpose

Define terminal viewer rendering, navigation, popups, help, search prompt, and reload behavior for the Rust `tabview` TUI.

## Requirements

### Requirement: Ratatui terminal viewer
The system SHALL render a spreadsheet-like terminal viewer using Ratatui and crossterm while preserving the original Tabview screen structure as closely as practical.

#### Scenario: Initial screen layout
- **WHEN** a file is opened
- **THEN** the screen shows the current cell location at the top left, the current cell contents next to it, a divider line, an optional header row, and visible table cells below

### Requirement: Navigation key compatibility
The system SHALL support the existing navigation keybindings, including cursor keys, `h/j/k/l`, `J/K`, `H/L`, `g`, `[num]G`, `[num]|`, `Home`, `End`, `^`, `$`, `Ctrl-a`, `Ctrl-e`, mark and return-to-mark.

#### Scenario: Vim movement
- **WHEN** a user presses `j`, `k`, `h`, or `l`
- **THEN** the highlighted cell moves down, up, left, or right respectively while keeping the viewport valid

#### Scenario: Numeric row jump
- **WHEN** a user presses `12G`
- **THEN** the highlighted cell moves to row 12 in the current column

### Requirement: Modal popups
The system SHALL provide modal popups for full cell contents, file/data information, and help.

#### Scenario: Full cell popup
- **WHEN** a user presses Enter on a non-empty cell
- **THEN** a scrollable popup displays the full cell contents and can be closed with Enter or `q`

#### Scenario: Empty cell popup
- **WHEN** a user presses Enter on an empty cell
- **THEN** no blank popup is opened and the viewer continues running

### Requirement: Search prompt interaction
The system SHALL provide an incremental search prompt compatible with `/`, `n`, and `p` behavior.

#### Scenario: Incremental search
- **WHEN** a user opens search with `/` and types printable characters
- **THEN** the highlighted cell updates to the next matching cell as the query changes

### Requirement: Dynamic help
The system SHALL render help text from the active keybinding registry rather than duplicating static keybinding text.

#### Scenario: Help reflects registry
- **WHEN** a user presses F1 or `?`
- **THEN** the help popup lists the commands and keys currently registered by the application

### Requirement: Reload state preservation
The system SHALL preserve current cursor position, column width mode, column gap, per-column widths, and active search string across reload when the user presses `r`.

#### Scenario: Reload changed file
- **WHEN** a user changes column widths, searches for text, and presses `r`
- **THEN** the file is reloaded and the preserved view state is reapplied to the new table where possible
