## ADDED Requirements

### Requirement: Composable interactive and output options
The Rust executable SHALL accept `--interactive`/`-i` as a runtime-mode flag and `--output <format>`/`-o <format>` as a serialization-format option. This change SHALL support `table`; future values such as `csv` and `markdown` SHALL extend the output format without becoming runtime modes. With neither option, Tabview SHALL retain automatic terminal detection.

#### Scenario: Explicit table output
- **WHEN** a user runs `tabview -o table data.json`
- **THEN** the command writes one formatted table to stdout and exits without starting the TUI

#### Scenario: Explicit view-only interaction
- **WHEN** a user runs `tabview -i data.csv`
- **THEN** the command starts the interactive viewer and does not serialize the final live view after quitting

#### Scenario: View-only interaction with redirected stdout
- **WHEN** a user runs `tabview -i data.csv > unused.txt` from a controlling terminal without `--output`
- **THEN** Tabview uses the controlling terminal for the viewer and leaves redirected stdout empty

#### Scenario: Automatic interactive output is view-only
- **WHEN** a user runs `tabview data.csv` with terminal stdout and neither output option
- **THEN** the command starts the interactive viewer and does not serialize the final live view after quitting

#### Scenario: Composed interactive table transform
- **WHEN** a user runs `tabview -i -o table data.csv > edited.txt` from a controlling terminal
- **THEN** Tabview uses the controlling terminal for interaction and writes only the final live view to redirected stdout after a normal quit

#### Scenario: Future composed CSV transform
- **WHEN** a future CSV adapter is available and a user runs `tabview -i -o csv data.csv > edited.csv`
- **THEN** the same interactive runtime writes the final live view through the CSV adapter without changing the meaning of `-i`

#### Scenario: Invalid output value
- **WHEN** a user supplies an unsupported `--output` value
- **THEN** argument parsing rejects the invocation and lists the currently supported values without preventing new adapter values from being added later

#### Scenario: Redirect without explicit option
- **WHEN** a user runs `tabview data.csv > table.txt` without `--output`
- **THEN** automatic runtime resolution selects default `table` output

### Requirement: Color mode option
The Rust executable SHALL accept `--color auto|always|never` and use `auto` when omitted, with table-mode color disabled unless `always` is explicitly selected.

#### Scenario: Force colored table
- **WHEN** a user runs `tabview --output table --color always data.json`
- **THEN** stdout contains theme-derived ANSI table styling

#### Scenario: Force plain table
- **WHEN** a user runs `tabview --color never data.csv` with redirected stdout
- **THEN** stdout contains no ANSI escape sequences

#### Scenario: Invalid color value
- **WHEN** a user supplies an unsupported `--color` value
- **THEN** argument parsing rejects the invocation and lists the supported values

### Requirement: Pipeline-compatible standard input
The existing `-` input mode SHALL compose with runtime and format selection so piped input remains interactive when stdout is a terminal, becomes non-interactive when stdout is redirected or piped under automatic mode, and can be interactively transformed when `-i` and `-o <format>` are combined.

#### Scenario: Piped input to interactive viewer
- **WHEN** a user runs `producer | tabview -` with terminal stdout and default output mode
- **THEN** Tabview materializes or opens stdin data and starts the interactive viewer

#### Scenario: Piped conversion
- **WHEN** a user runs `producer | tabview - | consumer`
- **THEN** Tabview reads source data from stdin and writes a plain formatted table to stdout without competing for terminal input

#### Scenario: Piped interactive transformation
- **WHEN** a user runs `producer | tabview -i -o table - > transformed.txt` from a controlling terminal
- **THEN** Tabview drains stdin as table data, uses the controlling terminal for UI events and drawing, and writes the final live view to `transformed.txt` after normal quit
