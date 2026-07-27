## MODIFIED Requirements

### Requirement: Input source support
The system SHALL represent positional source targets as filesystem paths, `file://` URI paths, standard input, or parsed remote URLs and SHALL pass the target to the resolved format adapter without interpreting a remote URL as a local path.

#### Scenario: File URI path
- **WHEN** a user runs `tabview file:///tmp/data.csv`
- **THEN** the system reads `/tmp/data.csv`

#### Scenario: Standard input target
- **WHEN** a user runs `tabview -`
- **THEN** the system treats standard input as the source byte stream

#### Scenario: Remote URL target
- **WHEN** a user supplies a syntactically valid non-file URL
- **THEN** the system retains its scheme, authority, path, and safe display form for adapter resolution

#### Scenario: Remote URL is not a path
- **WHEN** an HTTP(S) or `libsql://` target is parsed
- **THEN** Tabview does not call local filesystem metadata or file-opening operations for that target

## ADDED Requirements

### Requirement: URL scheme format inference
Format resolution SHALL use explicit CLI or saved format first, then an unambiguous registered URL-scheme mapping, then existing local signature, extension, and bounded content probing. It SHALL NOT infer a source format from an ambiguous remote scheme.

#### Scenario: Explicit format wins
- **WHEN** a target has a recognized scheme and the user supplies a compatible explicit format
- **THEN** the explicit format selects the adapter

#### Scenario: LibSQL scheme
- **WHEN** a target uses `libsql://` and no format is supplied
- **THEN** format resolution selects SQLite before the SQLite adapter reports whether remote execution is supported

#### Scenario: Ambiguous HTTPS scheme
- **WHEN** an HTTPS target has no explicit or saved format
- **THEN** Tabview requires format selection instead of guessing Elasticsearch, JSON, or another HTTP-backed source

#### Scenario: Local format probing remains
- **WHEN** the target is a local path without an explicit format
- **THEN** existing signature, extension, and bounded content probing behavior remains authoritative
