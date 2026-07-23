## Purpose

Define the supported command-line compatibility surface for the Rust `tabview` replacement.

## Requirements

### Requirement: Replacement binary name
The Rust implementation SHALL install and run as a `tabview` executable.

#### Scenario: User invokes tabview
- **WHEN** a user runs `tabview <filename>` after installing the Rust package
- **THEN** the Rust executable opens the target file in the terminal viewer

### Requirement: Existing CLI arguments
The Rust executable SHALL accept the existing command-line interface: positional filename, `-` for stdin, `--encoding`/`-e`, `--delimiter`/`-d`, `--quoting`, `--start_pos`/`-s`, `--width`/`-w`, `--double_width`, `--quote-char`/`-q`, and extra classic start-position arguments in `+y:x` form, plus `--format`, `--json-path`, and `--schema-scan` source options.

#### Scenario: Current README invocation remains valid
- **WHEN** a user runs `tabview sample/data_ohlcv.csv --start_pos 6,5 --encoding utf-8`
- **THEN** the command is accepted and the viewer starts at row 6, column 5 using the requested encoding

#### Scenario: Classic start position remains valid
- **WHEN** a user runs `tabview sample/data_ohlcv.csv +6:5`
- **THEN** the viewer starts at row 6, column 5

#### Scenario: Existing CSV options remain valid
- **WHEN** a user supplies existing delimiter, quoting, quote-character, or encoding options for delimited input
- **THEN** those options retain their established meaning

### Requirement: Input format option
The Rust executable SHALL accept `--format auto|delimited|json|ndjson`, using `auto` by default, and SHALL reject incompatible format-specific argument combinations clearly.

#### Scenario: Force JSON format
- **WHEN** a user runs `tabview --format json response.data`
- **THEN** the JSON adapter is selected without relying on the filename extension

#### Scenario: Force delimited format
- **WHEN** a `.json`-named file actually contains delimited data and the user runs `tabview --format delimited data.json`
- **THEN** the delimited adapter is selected

#### Scenario: Incompatible delimiter option
- **WHEN** a user combines `--format json` with `--delimiter`
- **THEN** argument or source-option validation rejects the incompatible combination with a clear error

### Requirement: JSON starting-path option
The Rust executable SHALL accept `--json-path <pointer>` using RFC 6901 syntax and SHALL apply it before JSON table construction.

#### Scenario: Select Elasticsearch hits
- **WHEN** a user runs `tabview --format json --json-path /hits/hits response.json`
- **THEN** the embedded search-hit array is used as the table

#### Scenario: Invalid JSON pointer
- **WHEN** `--json-path` is not a valid RFC 6901 JSON Pointer
- **THEN** the invocation fails with a clear validation error

#### Scenario: JSON path with non-JSON format
- **WHEN** a user supplies `--json-path` while explicitly selecting `delimited`
- **THEN** validation rejects the incompatible source options

### Requirement: Schema scan option
The Rust executable SHALL accept `--schema-scan default|full`, using the bounded format default when omitted, with explicit CLI values overriding matching saved-view values.

#### Scenario: Force full JSON schema scan
- **WHEN** a user runs `tabview --schema-scan full records.ndjson`
- **THEN** the selected structured adapter scans through the selected table's end before marking its schema complete

#### Scenario: Restore default scan for invocation
- **WHEN** a saved view requests a full scan and the user supplies `--schema-scan default`
- **THEN** the invocation uses the bounded default schema scan policy

### Requirement: Default column width mode
The Rust executable SHALL use `mode` as the default column width mode when `--width` is not provided.

#### Scenario: Width omitted
- **WHEN** a user runs `tabview sample/data_ohlcv.csv` without `--width`
- **THEN** the viewer computes variable column widths using mode-based sizing

### Requirement: Python-style quoting names
The Rust executable SHALL accept Python CSV quoting names used by the existing CLI, including `QUOTE_MINIMAL`, `QUOTE_NONNUMERIC`, `QUOTE_ALL`, and `QUOTE_NONE`.

#### Scenario: MySQL pager quoting mode
- **WHEN** a user runs `tabview -d '\t' --quoting QUOTE_NONE -`
- **THEN** the command is accepted and stdin is parsed with tab delimiters and no quote interpretation

### Requirement: Standard input mode
The Rust executable SHALL support `-` as the filename to read data from standard input while still allowing interactive terminal input for the TUI.

#### Scenario: Pipe into tabview
- **WHEN** data is piped into `tabview -`
- **THEN** the data is loaded from stdin and the TUI remains interactive after loading

### Requirement: Python import API removal
The Rust rewrite SHALL NOT provide or promise compatibility for `import tabview` or `tabview.view(...)`.

#### Scenario: Documentation describes supported surface
- **WHEN** users read the Rust rewrite installation and usage documentation
- **THEN** the documented supported interface is the `tabview` CLI, not a Python module API

### Requirement: Saved view CLI overrides
When compiled with the `saved-views` feature, the Rust executable SHALL accept saved view override arguments that force a named saved view or disable saved view application for the current invocation.

#### Scenario: Force saved view
- **WHEN** a user runs `tabview --view cat-shards sample/data.csv`
- **THEN** the command is accepted and saved view selection uses the saved view named `cat-shards`

#### Scenario: Force saved view with extension
- **WHEN** a user runs `tabview --view cat-shards.yml sample/data.csv`
- **THEN** the command is accepted and saved view selection uses the saved view named `cat-shards`

#### Scenario: Disable saved views
- **WHEN** a user runs `tabview --no-view sample/data.csv`
- **THEN** the command is accepted and saved view discovery and application are skipped

#### Scenario: Conflicting saved view flags
- **WHEN** a user runs `tabview --view cat-shards --no-view sample/data.csv`
- **THEN** argument parsing rejects the invocation with a clear error

#### Scenario: Saved views feature disabled
- **WHEN** the binary is compiled without the `saved-views` feature
- **THEN** the saved view override arguments are not part of the supported command-line surface
