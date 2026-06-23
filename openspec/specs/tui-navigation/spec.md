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

### Requirement: Footer message line
The system SHALL provide a VI-style footer notification/message line for non-fatal warnings and errors.

#### Scenario: Saved view warning displayed
- **WHEN** saved view loading records a non-fatal warning before the TUI starts
- **THEN** the viewer displays the warning on the footer message line after loading

#### Scenario: Saved view warning logged
- **WHEN** saved view loading records a non-fatal warning before the TUI starts
- **THEN** the system also logs the warning for diagnostics outside the TUI

#### Scenario: Message line does not corrupt layout
- **WHEN** a footer message is displayed
- **THEN** the table viewport and footer remain coherent and navigable within the terminal dimensions

### Requirement: Saved view modal
When compiled with the `saved-views` feature, the system SHALL bind `v` to a modal that displays the current view configuration and save target.

#### Scenario: Open saved view modal for loaded view
- **WHEN** a saved view was loaded from disk and the user presses `v`
- **THEN** the viewer opens a modal showing the current view YAML and the loaded saved view filename

#### Scenario: Open saved view modal for placeholder view
- **WHEN** no saved view was loaded and the user presses `v`
- **THEN** the viewer opens a modal showing the current view YAML and a placeholder filename based on the opened input basename with a `.yml` extension

#### Scenario: Saved view modal unavailable with no-view
- **WHEN** the user invoked `tabview --no-view data.csv`
- **THEN** the `v` binding is unavailable for that session

#### Scenario: Close saved view modal
- **WHEN** the saved view modal is open and the user presses `Esc`
- **THEN** the modal closes without saving changes

#### Scenario: Save from saved view modal
- **WHEN** the saved view modal is open, the target file does not exist, and the user presses `s`
- **THEN** the viewer saves the displayed current view configuration immediately and shows a success message on the footer notification line

#### Scenario: Confirm overwrite
- **WHEN** saving from the saved view modal would overwrite an existing saved view file
- **THEN** the viewer asks for overwrite confirmation using `y` and `n` before writing the file

#### Scenario: Scroll saved view modal
- **WHEN** the displayed YAML is larger than the saved view modal viewport
- **THEN** the modal allows scrolling through the read-only YAML content
