## ADDED Requirements

### Requirement: Single Rust crate and binary
The rewrite SHALL be implemented as one Rust crate that builds one `tabview` binary.

#### Scenario: Cargo install
- **WHEN** a user installs the crate with `cargo install`
- **THEN** the installed executable is named `tabview`

### Requirement: Supported target environments
The Rust binary SHALL target macOS, Linux, and WSL for the first release.

#### Scenario: WSL terminal
- **WHEN** a user runs `tabview` inside WSL with a supported terminal
- **THEN** terminal rendering and keyboard input use the crossterm backend without requiring curses

### Requirement: Typestate boundaries
The implementation SHALL use typestate or equivalent compile-time state separation where it prevents invalid construction of decoded input, parsed rows, classified table models, or active terminal sessions.

#### Scenario: Parsed table construction
- **WHEN** code constructs a table model for the viewer
- **THEN** it can only do so from rows that have completed decoding, parsing, and rectangular normalization

### Requirement: Compatibility test harness
The implementation SHALL include tests that compare the Rust behavior against the existing Python implementation for selected compatibility surfaces before the hard replacement is completed.

#### Scenario: Parsing fixture comparison
- **WHEN** compatibility tests run against existing sample files
- **THEN** Rust parsed rows match the Python implementation unless the difference is marked as an accepted bug fix

#### Scenario: Navigation fixture comparison
- **WHEN** compatibility tests replay key sequences over small tables
- **THEN** Rust cursor and viewport state matches the expected Python-compatible state

### Requirement: Ratatui render tests
The implementation SHALL include render-level tests for important TUI states using Ratatui buffers or equivalent terminal snapshots.

#### Scenario: Header layout snapshot
- **WHEN** the viewer renders a table with a detected header row
- **THEN** the render test verifies the location bar, divider, header row, and selected cell placement

### Requirement: Feature exclusions
The implementation SHALL NOT add editing, formulas, filtering, or other data manipulation features beyond the current viewer operations.

#### Scenario: User attempts to edit a cell
- **WHEN** a user presses ordinary printable keys outside of search entry
- **THEN** the viewer does not modify table cell contents
