## Purpose

Define implementation architecture, target environments, test harness expectations, and excluded feature areas for the Rust `tabview` replacement.
## Requirements
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

### Requirement: Rust test suite
The implementation SHALL include Rust tests for CLI parsing, data ingestion, table operations, rendering, and terminal lifecycle boundaries.

#### Scenario: Parsing fixture coverage
- **WHEN** Rust tests run against existing sample files
- **THEN** parsed rows match the expected fixture behavior

#### Scenario: Navigation fixture coverage
- **WHEN** Rust tests replay key sequences over small tables
- **THEN** Rust cursor and viewport state matches the expected viewer state

### Requirement: Ratatui render tests
The implementation SHALL include render-level tests for important TUI states using Ratatui buffers or equivalent terminal snapshots.

#### Scenario: Header layout snapshot
- **WHEN** the viewer renders a table with a detected header row
- **THEN** the render test verifies the location bar, divider, header row, and selected cell placement

### Requirement: Feature exclusions
The implementation SHALL NOT add editing, formulas, or persistent data mutation features beyond current viewer operations. Filtering SHALL be allowed only as a viewer row-visibility operation that does not mutate parsed cell data.

#### Scenario: User attempts to edit a cell
- **WHEN** a user presses ordinary printable keys outside of search or filter entry
- **THEN** the viewer does not modify table cell contents

