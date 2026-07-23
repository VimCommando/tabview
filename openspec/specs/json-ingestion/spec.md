## Purpose

Define JSON and NDJSON source selection, row construction, schema discovery, and stable column behavior.

## Requirements

### Requirement: JSON and NDJSON input support
The system SHALL open JSON documents and newline-delimited JSON streams as tabular sources through dedicated adapters.

#### Scenario: JSON extension
- **WHEN** a user opens a `.json` file without forcing another format
- **THEN** the JSON adapter parses the selected JSON table

#### Scenario: NDJSON extension
- **WHEN** a user opens a `.ndjson` or `.jsonl` file without forcing another format
- **THEN** the NDJSON adapter treats each complete JSON document as a logical input record

#### Scenario: Explicit format resolves ambiguity
- **WHEN** extension or content probing cannot reliably distinguish JSON, NDJSON, and delimited data
- **THEN** the user can explicitly select the intended adapter

### Requirement: JSON starting path
The JSON adapter SHALL accept an RFC 6901 JSON Pointer that selects the node used as the table starting point while ignoring surrounding document metadata.

#### Scenario: Root table default
- **WHEN** no JSON starting path is configured
- **THEN** the adapter uses the document root as the selected table node

#### Scenario: Elasticsearch search hits
- **WHEN** a user opens an Elasticsearch `_search` response with JSON starting path `/hits/hits`
- **THEN** the elements of the embedded `hits.hits` array become table rows and surrounding metadata is not exposed as columns

#### Scenario: NDJSON record starting path
- **WHEN** an NDJSON starting path is configured
- **THEN** the pointer is resolved independently within each logical JSON document and the selected object or array is used as that document's row

#### Scenario: Escaped pointer segment
- **WHEN** a starting path contains a key requiring JSON Pointer escaping
- **THEN** the adapter resolves `~0` and `~1` according to RFC 6901

#### Scenario: Missing starting path
- **WHEN** the configured JSON starting path does not exist
- **THEN** the system reports a clear source-opening error before entering the interactive viewer

#### Scenario: Non-tabular selected value
- **WHEN** the selected node is neither an object nor an array
- **THEN** the system reports that the JSON starting path does not identify a tabular value

### Requirement: JSON row construction
The JSON adapter SHALL construct rows from selected array elements or from a selected object and SHALL preserve structured values without exploding nested arrays into additional rows or columns.

#### Scenario: Array of objects
- **WHEN** the selected node is an array of objects
- **THEN** each array element becomes a row and object leaf paths define columns

#### Scenario: Selected object
- **WHEN** the selected node is an object
- **THEN** the object is represented as one row

#### Scenario: Nested objects
- **WHEN** a row contains nested objects
- **THEN** the adapter recursively flattens object leaf values into columns identified by pointers relative to the row root

#### Scenario: Nested array value
- **WHEN** a row field contains an array
- **THEN** the array remains one structured JSON cell value rather than expanding table cardinality or schema width

#### Scenario: Array row
- **WHEN** a selected table contains array-valued rows instead of object-valued rows
- **THEN** the adapter maps row positions to stable generated columns

### Requirement: Bounded JSON schema discovery
The JSON and NDJSON adapters SHALL discover columns from at most the first 100 MiB of the selected table payload by default, finishing the logical row that crosses the boundary, and SHALL support an explicit full schema scan.

#### Scenario: Default bounded scan
- **WHEN** selected JSON table data exceeds the default schema scan limit
- **THEN** the adapter stops initial discovery after completing the logical row that crosses 100 MiB and marks the schema provisional

#### Scenario: Small input completes schema
- **WHEN** the selected table ends before the default scan limit
- **THEN** initial discovery reaches the end and marks the schema complete

#### Scenario: Full schema scan
- **WHEN** full schema scanning is selected by CLI or saved view
- **THEN** the adapter scans every selected logical row before marking the schema complete

#### Scenario: Independent thresholds
- **WHEN** schema discovery and store selection use byte thresholds
- **THEN** the schema scan limit and lazy-store threshold are represented as distinct configuration values even if both default to 100 MiB

### Requirement: JSON column identity and display names
JSON object columns SHALL use case-sensitive canonical JSON Pointers relative to the row root as source identity and SHALL use compact path-derived display labels by default.

#### Scenario: Unique leaf label
- **WHEN** a discovered path has a leaf key that is unique among initially discovered columns
- **THEN** the leaf key is used as its default display label

#### Scenario: Colliding leaf labels
- **WHEN** initially discovered paths share the same leaf key
- **THEN** parent segments are prepended until each display label is unique

#### Scenario: Path-like key
- **WHEN** an object key contains dots, slashes, brackets, or other path-like characters
- **THEN** canonical JSON Pointer identity remains unambiguous and the friendly label uses unambiguous escaping or bracket notation

#### Scenario: Full path remains inspectable
- **WHEN** a compact display label omits parent segments
- **THEN** column information exposes the full canonical source pointer

### Requirement: Late JSON columns
Columns discovered after initial schema construction SHALL append in first-seen order without renaming, reordering, or changing the identity of existing columns.

#### Scenario: New path after bounded scan
- **WHEN** incremental indexing encounters a previously unseen object leaf path
- **THEN** the adapter appends a new column and pads earlier rows with null for that column

#### Scenario: Late label collision
- **WHEN** a late column's leaf label conflicts with an existing display label
- **THEN** the new column receives the shortest non-conflicting path-derived label and the existing label remains unchanged

#### Scenario: Repeated path
- **WHEN** a later row contains an already known canonical path
- **THEN** its value maps to the existing column rather than creating another column

#### Scenario: Late type widening
- **WHEN** later typed values require a column's inferred source type to widen
- **THEN** the column type metadata widens without changing column identity or source order
