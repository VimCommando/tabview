## ADDED Requirements

### Requirement: Selectable multi-table source
A source containing multiple user-visible tables or views SHALL expose its catalog and allow exactly one selectable table to be opened lazily as an `OpenedTable`. Catalog entries MAY be unavailable for selection when the adapter retains an actionable capability reason.

#### Scenario: Relational source awaits selection
- **WHEN** an adapter discovers multiple selectable tables or views and merged source options do not select one
- **THEN** `OpenedSource` exposes the catalog and a source-owned selection mechanism without opening row stores for every candidate

#### Scenario: Relational source selection
- **WHEN** the application selects one selectable catalog entry
- **THEN** the source opens exactly that table or view as an `OpenedTable`

#### Scenario: Unavailable relational entry
- **WHEN** an adapter discovers a user-visible relation that it cannot query safely or compatibly
- **THEN** the catalog may retain the entry as unavailable with a reason without allowing it to create an `OpenedTable`

#### Scenario: Existing implicit source is opened
- **WHEN** a delimited, JSON, or NDJSON adapter opens its single implicit table
- **THEN** it continues to produce one selected `OpenedTable` through the same contract

### Requirement: Relational column source identity
Columns supplied by a relational adapter SHALL use stable source identity containing the selected table, source ordinal, and source name.

#### Scenario: Duplicate relational names
- **WHEN** a table exposes two columns with the same source name
- **THEN** their distinct source ordinals keep their source identities and runtime column IDs distinct

#### Scenario: Relational reload
- **WHEN** the same compatible table is reopened into a new source generation
- **THEN** durable column configuration is remapped through relational source identity rather than stale generation-scoped column IDs

#### Scenario: Database metadata bypasses delimited header inference
- **WHEN** a relational adapter supplies column metadata independently of result rows
- **THEN** the first result row remains data and the supplied definitions are used without delimited header classification

### Requirement: Source query and view transform boundary
The table model SHALL represent source operations separately from view operations. A `SourceQuery` SHALL contain source-native filters, source-native sort keys, and a row limit; a `ViewTransform` SHALL contain source-independent filters and sort keys applied only to the active source result.

#### Scenario: Source query replacement
- **WHEN** a source filter, source sort key, or source limit changes
- **THEN** the store executes a replacement source query and atomically publishes its result when complete

#### Scenario: View transform replacement
- **WHEN** only a view filter or view sort key changes
- **THEN** the viewer recomputes the derived view from the active source result without reopening or expanding the source query

#### Scenario: Stable column operands
- **WHEN** either operation layer references a column
- **THEN** it uses stable column identity independently of display label and visible position

### Requirement: Bounded source result
An opened table SHALL expose an active source result bounded by its source query, and source-query execution SHALL NOT fall back to unbounded materialization when a store cannot execute an operation.

#### Scenario: Store supports source query
- **WHEN** a store accepts a complete source query
- **THEN** it returns no more rows than the query limit and records the result extent

#### Scenario: Store rejects source query
- **WHEN** a store cannot execute a requested source filter or source sort
- **THEN** the operation is reported as unsupported and the previous successful result remains active

#### Scenario: Local view execution
- **WHEN** a view transform is applied
- **THEN** the canonical local executor may materialize only the already bounded active source result

### Requirement: Source result metadata and provenance
An active source result SHALL report whether it is known complete or limited, and MAY expose a source-native query artifact for inspection and reuse.

#### Scenario: Result reaches source end
- **WHEN** fewer rows satisfy the source query than its limit and the store reaches the end
- **THEN** the result extent is `Complete`

#### Scenario: Result reaches limit
- **WHEN** the store returns the configured number of rows without proving source exhaustion
- **THEN** the result extent is `Limited`

#### Scenario: SQLite query provenance
- **WHEN** a SQLite source result is active
- **THEN** its metadata exposes the logical parameterized SQL and bound values plus a copyable SQL representation using the configured source limit

#### Scenario: File source has no query language
- **WHEN** a file-backed source does not have a native query artifact
- **THEN** result metadata omits query text without affecting source or view operations

### Requirement: Relational row identity
A relational store SHALL preserve stable row identity across successful source-query replacements when the selected table exposes a usable primary key or row identifier, and SHALL explicitly reset row-bound state when it cannot establish stable identity.

#### Scenario: Stable keyed table row
- **WHEN** a SQLite table has a usable primary key or `rowid`
- **THEN** the adapter derives row identity from that key and retains it through view transformations and compatible source-query replacements

#### Scenario: Keyless result
- **WHEN** a selected view or `WITHOUT ROWID` table does not expose a usable stable key
- **THEN** the result reports that stable row identity is unavailable and replacement invalidates cursor-following and marks tied to prior rows

#### Scenario: Duplicate key metadata
- **WHEN** a purported identity is not unique in the active result
- **THEN** the store does not silently use it as stable identity

### Requirement: Atomic asynchronous source replacement
Source-query execution SHALL be revisioned so slow or superseded asynchronous results cannot overwrite newer operation state.

#### Scenario: Current revision completes
- **WHEN** the latest source query completes successfully
- **THEN** its result, extent, provenance, cursor reconciliation, and view transform are published together

#### Scenario: Stale revision completes
- **WHEN** an earlier source query completes after a newer revision was requested
- **THEN** the stale result is discarded

#### Scenario: Replacement fails
- **WHEN** a replacement query fails
- **THEN** the prior successful source result and view remain usable while the error is reported
