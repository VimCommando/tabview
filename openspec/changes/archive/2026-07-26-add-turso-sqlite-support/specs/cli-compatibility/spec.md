## MODIFIED Requirements

### Requirement: Existing CLI arguments
The Rust executable SHALL accept the existing command-line interface: positional filename, `-` for stdin, `--encoding`/`-e`, `--delimiter`/`-d`, `--quoting`, `--start_pos`/`-s`, `--width`/`-w`, `--double_width`, `--quote-char`/`-q`, and extra classic start-position arguments in `+y:x` form, plus `--format`, `--json-path`, and `--schema-scan` source options. A build with the default-enabled `sqlite` feature SHALL additionally accept `--table`.

#### Scenario: Current README invocation remains valid
- **WHEN** a user runs `tview sample/data_ohlcv.csv --start_pos 6,5 --encoding utf-8`
- **THEN** the command is accepted and the viewer starts at row 6, column 5 using the requested encoding

#### Scenario: Classic start position remains valid
- **WHEN** a user runs `tview sample/data_ohlcv.csv +6:5`
- **THEN** the viewer starts at row 6, column 5

#### Scenario: Existing CSV options remain valid
- **WHEN** a user supplies existing delimiter, quoting, quote-character, or encoding options for delimited input
- **THEN** those options retain their established meaning

### Requirement: Input format option
The Rust executable SHALL accept `--format auto|delimited|json|ndjson`, using `auto` by default, and SHALL reject incompatible format-specific argument combinations clearly. A build with the default-enabled `sqlite` feature SHALL additionally accept `sqlite`.

#### Scenario: Force JSON format
- **WHEN** a user runs `tview --format json response.data`
- **THEN** the JSON adapter is selected without relying on the filename extension

#### Scenario: Force delimited format
- **WHEN** a `.json`-named file actually contains delimited data and the user runs `tview --format delimited data.json`
- **THEN** the delimited adapter is selected

#### Scenario: Force SQLite format
- **WHEN** a user runs `tview --format sqlite --table users application.data`
- **THEN** the local SQLite adapter is selected without relying on the filename extension

#### Scenario: SQLite feature is disabled
- **WHEN** a user runs a binary compiled without `sqlite`
- **THEN** `--format sqlite` is rejected as an unavailable format and `--table` is not exposed

#### Scenario: Incompatible delimiter option
- **WHEN** a user combines `--format json` with `--delimiter`
- **THEN** argument or source-option validation rejects the incompatible combination with a clear error

## ADDED Requirements

### Requirement: SQLite table selection argument
When compiled with the `sqlite` feature, the Rust executable SHALL accept
`--table <name>` to select a user-facing ordinary table or compatible ordinary
view from a SQLite input and SHALL report a classified unsupported object
distinctly from a missing name.

#### Scenario: Select SQLite table
- **WHEN** a user runs `tview application.db --table users`
- **THEN** the command opens the `users` relation when the input is SQLite and that relation exists

#### Scenario: Table option on non-relational input
- **WHEN** a user supplies `--table` for delimited, JSON, NDJSON, or stdin input
- **THEN** startup fails with a clear message that table selection requires a SQLite source

#### Scenario: Delimited option on SQLite input
- **WHEN** a user supplies `--encoding`, `--delimiter`, `--quoting`, or `--quote-char` for SQLite input
- **THEN** startup fails with a clear message identifying the incompatible option

### Requirement: Local SQLite CLI scope
The SQLite CLI surface SHALL accept local filesystem and `file://` inputs but SHALL NOT interpret stdin, Turso Cloud, or `libsql://` URLs as supported SQLite sources in this change.

#### Scenario: File URI database
- **WHEN** a user supplies a `file://` URI whose resolved file is selected as SQLite
- **THEN** the system opens it through the local SQLite adapter

#### Scenario: SQLite from stdin
- **WHEN** a user explicitly selects SQLite for `-`
- **THEN** the system reports that SQLite requires a local path

#### Scenario: Remote URL
- **WHEN** a user supplies a Turso Cloud or `libsql://` URL
- **THEN** the system reports that remote database access is unsupported rather than attempting a local open
