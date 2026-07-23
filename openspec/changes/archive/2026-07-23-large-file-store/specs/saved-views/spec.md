## MODIFIED Requirements

### Requirement: Saved view schema
The system SHALL ship and document a schema file that validates the supported saved view YAML structure, including `name`, top-level `locale`, `filenames`, source `format`, JSON `json_path`, `schema_scan`, view and column `nulls` policies, `columns`, column labels, column visibility, column type aliases, format values, width values, alignment values, numeric masks, sort state, and filter state.

#### Scenario: Editor validation
- **WHEN** a user configures a YAML language server with the shipped schema
- **THEN** valid saved view files using source options, JSON Pointer column keys, display labels, and the existing supported structure validate without schema errors

#### Scenario: Invalid enum value
- **WHEN** a saved view sets `format`, `schema_scan`, view or column `nulls`, a column `type`, or a column `format` to an unsupported value
- **THEN** schema validation reports the field as invalid

#### Scenario: Invalid JSON pointer
- **WHEN** a saved view supplies a `json_path` that is not a valid RFC 6901 JSON Pointer
- **THEN** semantic validation records a non-fatal saved-view warning and does not apply the invalid path

### Requirement: Column matching
The system SHALL apply column configuration sparsely using stable canonical source identity where available, with compatible header-label matching for delimited sources and unambiguous fallback matching for structured sources.

#### Scenario: Exact column key wins
- **WHEN** `columns` contains both `count` and `*count` and a compatible delimited table has a `Count` header
- **THEN** the system applies the exact `count` configuration to that column

#### Scenario: Wildcard column key
- **WHEN** `columns` contains `*count` and a compatible delimited table has `docs_count` and `store_count` headers
- **THEN** the system applies the wildcard configuration to both matching columns unless an exact configuration also exists

#### Scenario: Exact JSON pointer
- **WHEN** a JSON saved view configures canonical pointer `/_source/user/id`
- **THEN** the system matches that source column case-sensitively regardless of its compact display label

#### Scenario: Unambiguous JSON display label
- **WHEN** a JSON saved view uses a display label that identifies exactly one loaded column and no canonical source key matches
- **THEN** the system may apply that configuration as a compatibility fallback

#### Scenario: Ambiguous JSON display label
- **WHEN** a configured display label could refer to more than one structured source column
- **THEN** the system does not guess and records a non-fatal warning that recommends canonical source pointers

#### Scenario: Missing configured column
- **WHEN** a saved view configures a column key that matches no loaded column after a complete schema scan
- **THEN** the system ignores that column configuration and records a non-fatal warning

## ADDED Requirements

### Requirement: Saved source options
A saved view SHALL support format selection, JSON starting path, and schema scan policy as source-opening options applied before the table is opened.

#### Scenario: Saved JSON starting path
- **WHEN** a matching saved view sets `format: json` and `json_path: /hits/hits`
- **THEN** source opening selects that embedded JSON value before constructing table columns or rows

#### Scenario: Saved full schema scan
- **WHEN** a matching saved view sets `schema_scan: full`
- **THEN** JSON schema discovery scans all selected rows before the table schema is marked complete

#### Scenario: CLI source option precedence
- **WHEN** both a saved view and an explicit CLI argument provide the same source option
- **THEN** the explicit CLI value takes precedence for that invocation

### Requirement: Pending late-column configuration
The system SHALL retain valid canonical column configuration that does not match the initial provisional schema until the schema becomes complete or the column is discovered.

#### Scenario: Configured column arrives late
- **WHEN** a saved view configures a canonical JSON pointer absent from the bounded initial scan and that pointer is discovered during later indexing
- **THEN** the system applies the pending configuration when the column is appended

#### Scenario: Configured column never arrives
- **WHEN** schema discovery reaches the selected table's end without finding a pending canonical column
- **THEN** the system records the normal non-fatal missing-column warning

### Requirement: Column display-label override
A saved view SHALL allow a column to override its rendered display label without changing source identity or raw data.

#### Scenario: JSON column label
- **WHEN** canonical JSON column `/_source/user/email` sets `label: User email`
- **THEN** the fixed header and column information use `User email` as the display label while saved-view resolution retains the canonical pointer

#### Scenario: Duplicate label override
- **WHEN** label overrides create duplicate rendered labels
- **THEN** stable source identity remains distinct and ambiguous label-based configuration fallback is disabled for those columns

### Requirement: Saved null-placement policy
A saved view SHALL accept `nulls: first|last` at the view level and within individual column configuration, with column configuration overriding the view default and omission using the built-in `last` default.

#### Scenario: View default
- **WHEN** a saved view sets top-level `nulls: first`
- **THEN** every sorted column without an explicit column policy resolves to nulls first

#### Scenario: Column override
- **WHEN** a saved view sets top-level `nulls: first` and column `deleted_at` sets `nulls: last`
- **THEN** sorting `deleted_at` resolves to nulls last while other columns continue to inherit nulls first

#### Scenario: Column inherits view policy
- **WHEN** a column omits `nulls`
- **THEN** its column configuration retains inheritance rather than copying a fixed value, so a later view-default change affects it

#### Scenario: Pending structured column policy
- **WHEN** a provisional structured schema has pending canonical configuration with a `nulls` override
- **THEN** the override is applied when that column is discovered and is used by subsequent sorting

#### Scenario: Serialize null placement
- **WHEN** the view or a column has an explicit null-placement policy
- **THEN** displayed or saved view YAML includes the corresponding `nulls` field and omits it for an inheriting column
