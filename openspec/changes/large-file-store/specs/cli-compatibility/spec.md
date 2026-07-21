## MODIFIED Requirements

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

## ADDED Requirements

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
