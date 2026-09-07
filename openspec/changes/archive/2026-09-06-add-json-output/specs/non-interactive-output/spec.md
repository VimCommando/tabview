## ADDED Requirements

### Requirement: Structured display export
The system SHALL support explicit `--output json` and `--output jsonl` without changing automatic terminal detection or default table output. Both formats SHALL serialize complete projected display strings without clipping, padding, control replacement, or ANSI styling.

#### Scenario: JSON document
- **WHEN** JSON output completes successfully
- **THEN** stdout contains one newline-terminated object with ordered string `columns` and string-array `rows`

#### Scenario: JSONL records
- **WHEN** JSONL output completes successfully
- **THEN** each row produces one newline-terminated object with ordered string `columns` and string-array `values`

#### Scenario: Duplicate or absent labels
- **WHEN** labels repeat or the header is hidden or absent
- **THEN** positional values retain their order and columns is empty only for the hidden or absent header

#### Scenario: Empty result
- **WHEN** there are no visible rows
- **THEN** JSON emits an empty rows array and JSONL emits zero records

#### Scenario: Controls and late schema
- **WHEN** cells contain control characters or later input adds columns
- **THEN** serialization escapes controls and uses the completed schema for every record

#### Scenario: Styling and failures
- **WHEN** forced ANSI is requested or source preparation fails
- **THEN** structured output fails with empty stdout; broken pipes remain clean exits and other write failures may leave partial bytes

### Requirement: CLI version identification
The executable SHALL expose its manifest version through `--version` without requiring an input source.

#### Scenario: Print version
- **WHEN** the user invokes `tview --version`
- **THEN** stdout contains `tview` and the package version followed by a newline and the process exits with status zero
