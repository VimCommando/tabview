## ADDED Requirements

### Requirement: Object interpretation modes
The shared source option SHALL represent object interpretation as `auto`, `record`, or `entries` independently of input format. An object-capable adapter SHALL apply it after format-specific selection; the JSON adapter SHALL apply it after resolving the configured JSON Pointer. `auto` SHALL be the default, `record` SHALL preserve single-row flattened-object behavior, and `entries` SHALL force each direct object/map member to become one row.

#### Scenario: Format-independent option contract
- **WHEN** an object-capable structured adapter consumes merged source options
- **THEN** it receives the shared `ObjectMode` value without a JSON-specific CLI, saved-view, or source-option name

#### Scenario: Automatic keyed-object interpretation
- **WHEN** a regular JSON starting point selects an object and object mode is omitted or set to `auto`
- **THEN** the adapter applies keyed-object detection to the selected object before constructing its table

#### Scenario: Force record interpretation
- **WHEN** object mode is `record` for an object that would otherwise satisfy keyed-object detection
- **THEN** the selected object becomes one recursively flattened row using the existing object-record behavior

#### Scenario: Force entries interpretation
- **WHEN** object mode is `entries` for a selected object
- **THEN** every direct member becomes one row regardless of whether automatic detection would classify the object as keyed rows

#### Scenario: Starting path precedes interpretation
- **WHEN** a JSON Pointer selects a nested object
- **THEN** object-mode detection or forcing applies to that selected object rather than to its containing document

#### Scenario: Array table remains unchanged
- **WHEN** the selected regular JSON value is an array
- **THEN** its elements remain table rows and the object mode does not change array cardinality

#### Scenario: Automatic mode bypasses non-object values
- **WHEN** `auto` receives a selected array or scalar
- **THEN** the adapter preserves that value's existing interpretation without resolving an object mode, including the existing non-tabular error for a selected scalar

#### Scenario: Explicit mode rejects non-object values
- **WHEN** `record` or `entries` receives a selected array or scalar
- **THEN** the adapter reports that the explicit object mode is incompatible with the selected shape

#### Scenario: NDJSON cardinality remains unchanged
- **WHEN** NDJSON documents contain objects whose children resemble keyed records
- **THEN** each logical NDJSON document still contributes one row and object mode does not multiply that document into entry rows

### Requirement: Conservative keyed-object detection
In `auto` mode, the JSON adapter SHALL classify a selected object as keyed rows only when a bounded detection sample contains at least three entries, every sampled value is an object, and at least one direct child property with a consistent JSON value kind appears in at least 75 percent of the sampled values. The sample SHALL inspect no more than the first 64 entries or the first 1 MiB of encoded entry data, finishing the entry that crosses the byte bound.

These rules define the current default detector and MAY be improved in later versions. Explicit `record` or `entries` configuration SHALL bypass the detector.

#### Scenario: Repository map is detected
- **WHEN** a selected object contains at least three object-valued repository entries and every sampled child contains same-typed `type` and `settings` properties
- **THEN** `auto` mode classifies the object as keyed rows

#### Scenario: Scalar-bearing record stays one row
- **WHEN** a selected object contains scalar-valued direct properties
- **THEN** `auto` mode retains single-record interpretation

#### Scenario: Too few entries remain a record
- **WHEN** a selected object contains fewer than three direct entries
- **THEN** `auto` mode retains single-record interpretation even if every value is an object

#### Scenario: Heterogeneous child objects remain a record
- **WHEN** every sampled value is an object but no same-typed direct child property appears in at least 75 percent of them
- **THEN** `auto` mode retains single-record interpretation

#### Scenario: Explicit null differs from an absent property
- **WHEN** a direct child property is explicitly `null` in enough sampled entries to meet the shared-property threshold
- **THEN** those occurrences count as the same `null` value kind while entries where the property is absent do not count

#### Scenario: Large-object detection is bounded
- **WHEN** a selected object contains more than 64 entries or its sampled entries exceed 1 MiB
- **THEN** classification completes after the bounded sample without reading every map entry solely to choose an interpretation

### Requirement: Keyed-entry row projection
For keyed rows, the adapter SHALL preserve source member order, place the direct object key in the first column, recursively flatten object-valued entry data relative to the child object, and preserve native scalar, null, array, and structured value kinds.

#### Scenario: Object-valued entries become records
- **WHEN** `repositories.json` contains member `siem2` whose value has `type`, `uuid`, and nested `settings` properties
- **THEN** one row contains key value `siem2` followed by `type`, `uuid`, and flattened `settings` columns without prefixing them with `siem2`

#### Scenario: Optional child property
- **WHEN** a child property such as `uuid` is absent from one keyed entry
- **THEN** that row contains null for the shared `uuid` column

#### Scenario: Forced scalar-valued entries
- **WHEN** `entries` mode receives an object whose direct values are scalar, null, or array values
- **THEN** the adapter places each member key in the key column and its typed value in a `value` column

#### Scenario: Source order is stable
- **WHEN** keyed entries are rendered without an active sort
- **THEN** their row order matches the direct member order in the selected object/map

### Requirement: Unique keyed-entry member keys
An object/map interpreted as entries SHALL reject duplicate direct member keys rather than overwriting an earlier value or exposing multiple rows with ambiguous key identity.

#### Scenario: Materialized duplicate key
- **WHEN** a materialized selected object/map contains a duplicate direct member key
- **THEN** source opening fails with a clear duplicate-key error

#### Scenario: Incrementally discovered duplicate key
- **WHEN** a lazy keyed-object store encounters a duplicate key after initial rendering
- **THEN** indexing fails safely without overwriting the earlier row, emitting a second ambiguous row, or activating partial replacement state

### Requirement: Synthetic key-column identity and label
The keyed-entry key column SHALL have a format-neutral stable object-key source identity distinct from every structured child-path column, SHALL be addressable by canonical saved-view key `@key`, and SHALL default to display label `name`. Structured child paths SHALL use RFC 6901 JSON Pointer representation without coupling the shared identity implementation to the JSON adapter. If the initial child schema would also use display label `name`, the synthetic column SHALL instead use `_key`; late child columns SHALL receive a non-conflicting label without renaming an established key-column label.

#### Scenario: Default name column
- **WHEN** the initial keyed-entry child schema contains no column whose display label is `name`
- **THEN** the first column has canonical identity `@key`, display label `name`, and text values containing the direct object keys

#### Scenario: Initial name collision
- **WHEN** the initial child schema contains a property whose display label is `name`
- **THEN** the child retains `name` and the synthetic key column is labeled `_key`

#### Scenario: Saved key-column configuration
- **WHEN** a saved view configures column `@key`
- **THEN** that configuration applies to the synthetic key column independently of its current display label

### Requirement: Keyed-object schema evolution
Keyed-entry schema discovery SHALL use child-relative canonical JSON Pointers, existing bounded or full schema-scan policy, monotonic type widening, and append-only late-column behavior without treating direct object keys as path prefixes.

#### Scenario: Shared child columns
- **WHEN** multiple keyed entries contain the same child-relative path such as `/settings/client`
- **THEN** their values map to one stable source column

#### Scenario: Late property appends
- **WHEN** incremental indexing encounters a child-relative path absent from the initial bounded schema
- **THEN** the adapter appends that column, pads earlier keyed rows with null, and preserves established columns and labels

#### Scenario: Full schema scan
- **WHEN** full schema scanning is requested for a keyed object
- **THEN** every keyed entry contributes schema and type information before the schema is marked complete

### Requirement: Incremental keyed-object storage
Large seekable keyed objects SHALL support bounded schema discovery, initial rendering, indexed random row access, progressive navigation, source-generation validation, and controlled materialization without decoding the entire selected object into one in-memory JSON value.

#### Scenario: Large keyed object opens incrementally
- **WHEN** a seekable selected object exceeds the lazy-store threshold and `auto` or `entries` mode selects keyed rows
- **THEN** the store records member keys and value boundaries incrementally and renders the initial viewport without materializing every member

#### Scenario: Navigation indexes later entries
- **WHEN** navigation requests a keyed row beyond the indexed range
- **THEN** the store indexes map entries through the requested row or the selected object's end

#### Scenario: Indexed entry reparses independently
- **WHEN** the store reads an indexed keyed row
- **THEN** it seeks to that member's recorded value boundary, reparses the value, and combines it with the recorded member key

#### Scenario: Source changes during indexing
- **WHEN** a seekable keyed-object source changes incompatibly after opening
- **THEN** later indexing fails without mixing generations or activating partially decoded replacement state

### Requirement: Object-mode visibility and reload
The viewer SHALL expose the requested and resolved object mode in source/table information independently of input format and SHALL retain the selected mode across reload using normal CLI and saved-view precedence.

#### Scenario: Auto detection is inspectable
- **WHEN** `auto` resolves a selected object to keyed rows
- **THEN** source information reports an automatically detected entries interpretation and identifies `record` as the override for single-row behavior

#### Scenario: Explicit mode is inspectable
- **WHEN** `record` or `entries` is explicitly selected
- **THEN** source information reports that explicit mode

#### Scenario: Explicit saved mode bypasses detection
- **WHEN** a saved view supplies `record` or `entries`
- **THEN** that mode determines the selected object's shape without applying the current automatic detector

#### Scenario: Reload preserves interpretation
- **WHEN** a keyed JSON source is reloaded
- **THEN** the same effective object-mode source option is reapplied before table and saved-column configuration
