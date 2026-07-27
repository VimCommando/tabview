## Purpose

Define format-aware source opening and the stable, typed table definition and row-store contract.

## Requirements

### Requirement: Format-aware source opening
The system SHALL resolve an input format and open it through a format adapter rather than applying delimited parsing to every source.

#### Scenario: Explicit format selection
- **WHEN** a user explicitly selects a supported input format
- **THEN** the system uses that adapter without content-based format inference

#### Scenario: Automatic format selection
- **WHEN** no explicit or saved-view format is selected
- **THEN** the system resolves a supported adapter from the source name and bounded content probing
### Requirement: Opened table contract
Every opened table SHALL provide a table definition and a row store as separate responsibilities.

#### Scenario: Source constructs table definition
- **WHEN** an adapter opens a table
- **THEN** it supplies ordered column definitions, schema completeness, and source metadata without asking `TableView` to consume a data row as a header

#### Scenario: Store supplies rows
- **WHEN** the viewer requests table data
- **THEN** it obtains logical rows, row-count state, indexing progress, and materialization behavior through the store boundary
### Requirement: Stable column definitions
Each source column SHALL have stable internal identity, source identity, display name, type metadata, and first-seen source order as applicable to its format.

#### Scenario: Duplicate display names
- **WHEN** two columns have identical or ambiguous source names
- **THEN** stable column identity remains distinct from the rendered display name

#### Scenario: View changes presentation
- **WHEN** a saved view changes a column label, type interpretation, format, width, alignment, or visibility
- **THEN** the source identity and raw typed values remain unchanged
### Requirement: Generation-scoped row and column identity
Every opened relation SHALL have a source generation, every row SHALL have opaque identity within that generation, and derived state SHALL NOT apply generation-scoped identities to another generation.

#### Scenario: File-backed row identity
- **WHEN** a delimited, JSON, or NDJSON adapter identifies a logical source row
- **THEN** it assigns a row identity derived from that logical source position and preserves it in filtered or sorted results

#### Scenario: Reload creates a generation
- **WHEN** the source is reloaded
- **THEN** the system creates a new source generation, discards old row identities and query result stores, and re-resolves durable column configuration through source identity

#### Scenario: Source changes during incremental access
- **WHEN** an adapter detects that a seekable source was replaced, truncated, or changed incompatibly after its generation opened
- **THEN** it fails the affected operation without mixing versions or activating partial derived state and reports that reload is required
### Requirement: Typed cell values
The opened-table model SHALL preserve null, boolean, integer, floating-point, text, binary, and structured JSON distinctions until presentation formatting is applied.

#### Scenario: Null differs from empty text
- **WHEN** a source contains both a null value and an empty string
- **THEN** the table model represents them as distinct raw values

#### Scenario: Numeric value remains numeric
- **WHEN** JSON supplies a native integer or floating-point value
- **THEN** the table model preserves its numeric kind without first converting it to display text
### Requirement: Schema completeness and updates
The table definition SHALL represent whether its schema is complete or provisional and SHALL support append-only schema updates from incremental discovery.

#### Scenario: Bounded discovery is provisional
- **WHEN** an adapter stops schema discovery before the selected table ends
- **THEN** it marks the schema provisional

#### Scenario: End of table completes schema
- **WHEN** indexing or a full schema scan reaches the selected table's end
- **THEN** the adapter marks the schema complete

#### Scenario: Schema delta reaches the view
- **WHEN** incremental indexing discovers a new source column
- **THEN** the store reports an append-only schema delta so view metadata can be extended without rebuilding existing column identity
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
