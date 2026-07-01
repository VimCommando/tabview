## ADDED Requirements

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
