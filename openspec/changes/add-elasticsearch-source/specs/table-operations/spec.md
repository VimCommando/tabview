## ADDED Requirements

### Requirement: Native base query composition
For a query-native source, Source Configuration SHALL compose supported source filters, source sorting, and the hard source limit around the native base query using adapter-native semantics. It SHALL NOT parse or rewrite a complete native query using another source's grammar.

#### Scenario: Compose ES|QL source operations
- **WHEN** an Elasticsearch source filter, sort, or limit is applied
- **THEN** the adapter appends a parameterized ES|QL stage in the defined source-operation order

#### Scenario: Compose SQLite source operations
- **WHEN** a SQLite native query receives a source filter, sort, or limit
- **THEN** the adapter treats the row-producing SQL as a bounded derived-table input and applies parameterized outer SQL

#### Scenario: Native query already contains limiting behavior
- **WHEN** a user-supplied native query contains its own `LIMIT` or equivalent stage
- **THEN** that behavior remains part of the opaque base query and Tview still applies its final hard source-result limit

#### Scenario: Unsupported composition
- **WHEN** an adapter cannot represent a requested source operation safely over the native base query
- **THEN** Source Configuration rejects the operation without materializing an unbounded remote or local source

#### Scenario: Local view remains separate
- **WHEN** a view filter, view sort, presentation change, or search is applied
- **THEN** it operates only over the fixed active native-query result and does not modify or re-execute the native source query

### Requirement: Elasticsearch Source configuration
When the active source is Elasticsearch, Source Configuration SHALL expose the endpoint's safe identity, selected target or native query, mappings-backed result fields when available, ES|QL-native filter/sort capabilities, limit, partial/extent state, and query provenance.

#### Scenario: Mapping-aware configuration
- **WHEN** Elasticsearch was opened from a selected index or data stream
- **THEN** Source Configuration can select fields from its mapping and field-capability catalog

#### Scenario: Query-only configuration
- **WHEN** Elasticsearch was opened from a complete ES|QL query without `source.table`
- **THEN** Source Configuration uses the active result columns and does not require a reconstructed mapping target

#### Scenario: Apply ES|QL draft
- **WHEN** the user applies a valid Elasticsearch source draft
- **THEN** at most one asynchronous replacement starts and the prior result remains visible until success

### Requirement: Native source query output
When a query-native source is active, the user SHALL be able to inspect and copy its final native query, language, and bound parameters or an equivalent safe executable representation.

#### Scenario: SQLite query artifact
- **WHEN** the active query language is SQL
- **THEN** the query UI labels and displays SQL rather than assuming the artifact was application-generated

#### Scenario: Elasticsearch query artifact
- **WHEN** the active query language is ES|QL
- **THEN** the query UI labels and displays ES|QL and its value or identifier parameters

#### Scenario: Source operations change
- **WHEN** a native base query, source filter, source sort, target selection, or source limit changes
- **THEN** the displayed artifact is regenerated from the complete active source request

#### Scenario: View operations change
- **WHEN** only a view filter, view sort, presentation option, or search changes
- **THEN** the native query artifact remains unchanged and the output identifies those operations as local view behavior

#### Scenario: Secret redaction
- **WHEN** native query information is displayed, copied, logged, or included in an error
- **THEN** transport credentials and authorization headers are absent
