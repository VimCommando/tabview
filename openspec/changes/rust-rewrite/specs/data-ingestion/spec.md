## ADDED Requirements

### Requirement: Input source support
The system SHALL load tabular data from filesystem paths, `file://` URI paths, and standard input.

#### Scenario: File URI path
- **WHEN** a user runs `tabview file:///tmp/data.csv`
- **THEN** the system reads `/tmp/data.csv`

### Requirement: Encoding detection and override
The system SHALL use the provided encoding when `--encoding` is set and SHALL otherwise attempt the compatibility encoding set with specific encodings before permissive single-byte fallbacks. The compatibility set SHALL include locale encoding, `utf-8`, `utf-16`, `iso8859-1`, `iso8859-2`, `cp720`, and `latin-1`, with `latin-1` as a late fallback.

#### Scenario: Latin-1 sample file
- **WHEN** a Latin-1 file is opened without `--encoding`
- **THEN** the file is decoded using a compatible fallback encoding if more specific encodings do not match

#### Scenario: Explicit encoding wins
- **WHEN** a user passes `--encoding iso8859-1`
- **THEN** the system decodes input with `iso8859-1` rather than sniffing another encoding

### Requirement: CSV delimiter and quoting compatibility
The system SHALL parse CSV-like input using explicit delimiter, quote character, and quoting options when provided, and SHALL infer a delimiter when no delimiter is provided.

#### Scenario: Explicit delimiter
- **WHEN** a user passes `--delimiter '\t'`
- **THEN** the parser treats tab as the field delimiter

#### Scenario: Explicit quote character
- **WHEN** a user passes `--quote-char "'"`
- **THEN** the parser uses `'` as the quote character

### Requirement: Space-delimited normalization
The system SHALL preserve the current space-delimited behavior: normalize repeated whitespace using shell-like quote parsing and strip a leading `#` or `%` only from the first line when space-delimited data is detected.

#### Scenario: Annotated numeric sample
- **WHEN** the annotated numeric sample begins with `#` and uses aligned spaces
- **THEN** the first line comment marker is stripped and repeated spacing is normalized before parsing

### Requirement: Row normalization
The system SHALL normalize parsed rows to a rectangular table by padding shorter rows with empty cells.

#### Scenario: Uneven rows
- **WHEN** input rows contain different numbers of fields
- **THEN** all parsed rows expose the same column count and missing cells are empty strings

### Requirement: Lazy large-file support
The system SHALL support lazy or streaming-backed table access for very large files without requiring all rows to be loaded before the viewer can start, except when an operation requires full-table materialization or indexing.

#### Scenario: Large file opens incrementally
- **WHEN** a user opens a file large enough to trigger lazy loading
- **THEN** the viewer can begin displaying initial rows without first materializing the complete file into memory

#### Scenario: Full-table operation on lazy input
- **WHEN** a user invokes a full-table operation such as sort on a lazy input
- **THEN** the system materializes or indexes the required data in a controlled way without corrupting the current table state
