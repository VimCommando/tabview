## MODIFIED Requirements

### Requirement: Input format option
The Rust executable SHALL accept `--format auto|delimited|json|ndjson`, using `auto` by default, and SHALL reject incompatible format-specific argument combinations clearly. A build with the `sqlite` feature SHALL additionally accept `sqlite`, and a build with the `elasticsearch` feature SHALL additionally accept `elasticsearch`. When `--format` is omitted, an unambiguous registered URL scheme MAY resolve the effective format before existing local probing.

#### Scenario: Force JSON format
- **WHEN** a user runs `tview --format json response.data`
- **THEN** the JSON adapter is selected without relying on the filename extension

#### Scenario: Force delimited format
- **WHEN** a `.json`-named file actually contains delimited data and the user runs `tview --format delimited data.json`
- **THEN** the delimited adapter is selected

#### Scenario: Force SQLite format
- **WHEN** a user runs `tview --format sqlite --table users application.data`
- **THEN** the SQLite adapter is selected without relying on the filename extension

#### Scenario: Infer SQLite from LibSQL
- **WHEN** a user supplies a `libsql://` target without `--format`
- **THEN** the effective format resolves to SQLite before adapter capability validation

#### Scenario: Force Elasticsearch format
- **WHEN** a user runs `tview https://elastic.example:9200 --format elasticsearch`
- **THEN** the Elasticsearch adapter is selected without attempting content probing

#### Scenario: Ambiguous remote format
- **WHEN** a user supplies an HTTP(S) target without explicit or saved format
- **THEN** startup fails clearly rather than guessing a remote adapter

#### Scenario: SQLite feature is disabled
- **WHEN** a user runs a binary compiled without `sqlite`
- **THEN** `--format sqlite` is rejected as unavailable and `--table` is exposed only if another enabled source feature supports relation selection

#### Scenario: Feature-disabled format
- **WHEN** a user requests a format whose Cargo feature is disabled
- **THEN** that format is rejected as unavailable and its source-specific dispatch is absent

#### Scenario: Incompatible delimiter option
- **WHEN** a user combines `--format json` with `--delimiter`
- **THEN** argument or source-option validation rejects the incompatible combination with a clear error

### Requirement: Relation selection argument
When at least one relational or query-native source feature is enabled, the Rust executable SHALL accept `--table <name>` as a generic relation or target selector. The resolved adapter SHALL define which catalog entries are selectable and SHALL reject the option for non-relational sources.

#### Scenario: Select SQLite table
- **WHEN** a user runs `tview application.db --table users`
- **THEN** the command opens `users` when the input is SQLite and that relation is selectable

#### Scenario: Select Elasticsearch index
- **WHEN** a user runs `tview https://elastic.example:9200 --format elasticsearch --table application-events`
- **THEN** the command uses `application-events` as the generated bounded ES|QL `FROM` target without requiring picker discovery

#### Scenario: Select Elasticsearch data stream
- **WHEN** `--table` names a visible Elasticsearch data stream
- **THEN** the command selects the data stream as the ES|QL `FROM` target

#### Scenario: Select Elasticsearch alias
- **WHEN** `--table` names an Elasticsearch alias
- **THEN** the command passes the alias through as the ES|QL `FROM` target and lets Elasticsearch validate it

#### Scenario: Table option on non-relational input
- **WHEN** a user supplies `--table` for delimited, JSON, NDJSON, or stdin input
- **THEN** startup fails with a clear message that the resolved source does not support relation selection

#### Scenario: Delimited option on SQLite input
- **WHEN** a user supplies `--encoding`, `--delimiter`, `--quoting`, or `--quote-char` for SQLite input
- **THEN** startup fails with a clear message identifying the incompatible option

#### Scenario: Classified unsupported relation
- **WHEN** a selected adapter recognizes a name but classifies it as unavailable
- **THEN** startup reports the classified reason distinctly from a missing name

## ADDED Requirements

### Requirement: Generic native query option
When at least one native-query source feature is enabled, the Rust executable SHALL accept `--query <string>` and pass the string as the native query language selected by the resolved source format. `--query` and `--table` SHALL be mutually exclusive.

#### Scenario: Elasticsearch ES|QL
- **WHEN** a user supplies `--format elasticsearch --query 'FROM logs-* | LIMIT 10'`
- **THEN** the Elasticsearch adapter receives the string as complete ES|QL and does not display a target picker

#### Scenario: SQLite SQL
- **WHEN** a user supplies `--format sqlite --query 'SELECT id, name FROM users'`
- **THEN** the SQLite adapter receives the string as a confined row-producing SQL query

#### Scenario: Table and query conflict
- **WHEN** a user supplies both `--table` and `--query`
- **THEN** argument validation rejects the invocation before opening the source

#### Scenario: Query on non-native source
- **WHEN** a user supplies `--query` for delimited, JSON, NDJSON, or stdin input
- **THEN** startup fails with a clear message that the resolved source does not support native queries

#### Scenario: Query feature unavailable
- **WHEN** all compiled source adapters lack native-query support
- **THEN** `--query` is omitted from the compiled CLI surface

## RENAMED Requirements

- FROM: `### Requirement: SQLite table selection argument`
- TO: `### Requirement: Relation selection argument`
