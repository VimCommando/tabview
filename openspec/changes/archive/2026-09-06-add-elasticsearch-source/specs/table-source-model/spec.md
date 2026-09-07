## MODIFIED Requirements

### Requirement: Source result metadata and provenance
An active source result SHALL report whether it is known complete, limited, or partial and MAY expose a language-neutral source-native query artifact for inspection and reuse.

#### Scenario: Result reaches source end
- **WHEN** fewer rows satisfy the source query than its limit and the store reaches the end
- **THEN** the result extent is `Complete`

#### Scenario: Result reaches limit
- **WHEN** the store returns the configured number of rows without proving source exhaustion
- **THEN** the result extent is `Limited`

#### Scenario: Source reports partial execution
- **WHEN** a remote source reports that only a partial result was computed
- **THEN** result metadata preserves partial status independently of complete-or-limited extent

#### Scenario: SQLite query provenance
- **WHEN** a SQLite source result is active
- **THEN** its metadata identifies SQL as the native language and exposes logical text, bound values, and a copyable representation using the configured source limit

#### Scenario: Elasticsearch query provenance
- **WHEN** an Elasticsearch source result is active
- **THEN** its metadata identifies ES|QL as the native language and exposes logical text, bound value and identifier parameters, and a copyable representation

#### Scenario: File source has no query language
- **WHEN** a file-backed source does not have a native query artifact
- **THEN** result metadata omits query text without affecting source or view operations

### Requirement: Atomic asynchronous source replacement
Source-query execution SHALL be revisioned so slow or superseded asynchronous results cannot overwrite newer operation state, and a successful replacement SHALL atomically publish both the result schema and row store.

#### Scenario: Current revision completes
- **WHEN** the latest source query completes successfully
- **THEN** its result schema, store, extent, provenance, cursor reconciliation, and view transform are published together

#### Scenario: Current revision changes schema
- **WHEN** the latest native query completes successfully with added, removed, renamed, reordered, or retyped result columns
- **THEN** a new table definition and compatible remapped view state are published atomically with its store, extent, and provenance

#### Scenario: Stale revision completes
- **WHEN** an earlier source query completes after a newer revision was requested
- **THEN** the stale table definition and result are discarded together

#### Scenario: Replacement fails
- **WHEN** a replacement query fails
- **THEN** the prior successful table definition, source result, and view remain usable while the error is reported

## ADDED Requirements

### Requirement: Native query request model
The source model SHALL represent an optional native base query separately from generic source operations and SHALL let each adapter validate and compose its own query language without exposing source-specific CLI fields in the table model.

#### Scenario: Source has native query
- **WHEN** a SQLite or Elasticsearch source receives `source.query`
- **THEN** the adapter receives the unchanged configured text as its native base query

#### Scenario: Source has generated query
- **WHEN** a query-native source receives a selected relation and no native query
- **THEN** the adapter generates its native base query from resolved source metadata

#### Scenario: Source has no native language
- **WHEN** a file-backed adapter receives a native query
- **THEN** capability validation rejects it instead of interpreting the text as a file filter

### Requirement: Native query result definition
An adapter whose native query can shape columns SHALL construct `TableDefinition` from prepared or returned result metadata and SHALL assign a new source generation when replacement changes the result definition.

#### Scenario: SQLite prepared columns
- **WHEN** a native SQLite query is prepared successfully
- **THEN** its result column metadata defines the opened implicit table

#### Scenario: ES|QL response columns
- **WHEN** an ES|QL request succeeds
- **THEN** its returned column names and types define the active table

#### Scenario: Same names but changed types
- **WHEN** a replacement retains column names but changes source types
- **THEN** Tview treats the result definition as changed rather than retaining stale type metadata

#### Scenario: Saved column remapping
- **WHEN** a new result definition retains unambiguous compatible source identities
- **THEN** saved and active presentation state is remapped to those columns while stale identities are reported or omitted according to existing policy

### Requirement: Asynchronous source task kinds
The source-query coordinator SHALL support asynchronous network jobs and blocking local jobs through one revision contract without blocking the terminal event loop or creating an asynchronous call per rendered cell.

#### Scenario: Remote async job
- **WHEN** Elasticsearch performs discovery or ES|QL execution
- **THEN** its client future runs on the application Tokio runtime

#### Scenario: Blocking local job
- **WHEN** a local source operation uses blocking work
- **THEN** it runs through the runtime's blocking boundary while publishing the same revisioned completion event

#### Scenario: Render cached result
- **WHEN** the table body renders a completed remote source result
- **THEN** cells are read from the active store without issuing per-row or per-cell HTTP requests
