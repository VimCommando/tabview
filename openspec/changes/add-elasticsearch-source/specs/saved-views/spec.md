## ADDED Requirements

### Requirement: Saved native source query
The nested saved-view schema SHALL accept `source.query` as native query text interpreted by `source.format`, with explicit CLI `--query` taking precedence and `source.table` remaining mutually exclusive.

#### Scenario: Saved ES|QL
- **WHEN** a matching saved view sets `source.format: elasticsearch` and `source.query: FROM logs-* | LIMIT 25`
- **THEN** source opening executes that text as ES|QL without displaying the Elasticsearch target picker

#### Scenario: Saved SQLite SQL
- **WHEN** a matching saved view sets `source.format: sqlite` and a valid read-only `source.query`
- **THEN** source opening executes the query through the confined SQLite native-query path

#### Scenario: Saved table and query conflict
- **WHEN** a saved source contains both `table` and `query`
- **THEN** saved-view validation reports the conflict and does not choose one silently

#### Scenario: CLI query precedence
- **WHEN** a saved view contains `source.query` and the user supplies `--query`
- **THEN** the CLI query replaces the saved query for that invocation

#### Scenario: Serialize native query
- **WHEN** the active source was opened from a user-supplied native query
- **THEN** generated saved-view YAML persists the configured base query under `source.query` rather than only the derived composed artifact

### Requirement: Saved remote source target
Saved-view matching and serialization SHALL support the safe textual identity of a remote positional source target while excluding URL userinfo, credentials, authorization headers, and other transport secrets.

#### Scenario: Match Elasticsearch endpoint
- **WHEN** a saved view targets an Elasticsearch endpoint and its safe target pattern matches the invocation
- **THEN** normal saved-view selection and source-option merging apply

#### Scenario: Serialize remote target
- **WHEN** a saved view is generated for a remote source
- **THEN** its matching target uses the endpoint's safe non-secret representation

#### Scenario: Secret-bearing URL
- **WHEN** a supplied remote URL contains user information or another secret-bearing component
- **THEN** generated YAML, diagnostics, and query artifacts omit or redact that component

### Requirement: Saved native query serialization
Saved-view serialization SHALL persist source-native input configuration separately from derived query provenance.

#### Scenario: Source operations around native query
- **WHEN** a native base query has source filters, source sort, or source limit
- **THEN** YAML stores the base under `source.query` and structured operations under their existing source fields

#### Scenario: Derived ES|QL is excluded
- **WHEN** Elasticsearch query provenance includes application-composed stages
- **THEN** generated YAML does not duplicate the final composed ES|QL as a second configuration value

#### Scenario: Derived SQLite SQL is excluded
- **WHEN** SQLite query provenance includes an outer bounded query
- **THEN** generated YAML does not replace the configured base query with the derived SQL
