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
