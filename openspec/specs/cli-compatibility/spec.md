## Purpose

Define the supported command-line compatibility surface for the Rust `tabview` replacement.

## Requirements

### Requirement: Replacement binary name
The Rust implementation SHALL install and run as a `tabview` executable.

#### Scenario: User invokes tabview
- **WHEN** a user runs `tabview <filename>` after installing the Rust package
- **THEN** the Rust executable opens the target file in the terminal viewer

### Requirement: Existing CLI arguments
The Rust executable SHALL accept the existing command-line interface: positional filename, `-` for stdin, `--encoding`/`-e`, `--delimiter`/`-d`, `--quoting`, `--start_pos`/`-s`, `--width`/`-w`, `--double_width`, `--quote-char`/`-q`, and extra classic start-position arguments in `+y:x` form.

#### Scenario: Current README invocation remains valid
- **WHEN** a user runs `tabview sample/data_ohlcv.csv --start_pos 6,5 --encoding utf-8`
- **THEN** the command is accepted and the viewer starts at row 6, column 5 using the requested encoding

#### Scenario: Classic start position remains valid
- **WHEN** a user runs `tabview sample/data_ohlcv.csv +6:5`
- **THEN** the viewer starts at row 6, column 5

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
