## MODIFIED Requirements

### Requirement: Saved view schema
The system SHALL ship and document a schema for saved-view YAML with `name` and `filenames` at the document root, source-opening and source-query configuration under `source`, and source-independent presentation and local-operation configuration under `view`. `source` SHALL support `format`, `json_path`, `object_mode`, `table`, `schema_scan`, `limit`, `filters`, and `sort`. `view` SHALL support `locale`, `nulls`, `columns`, `filters`, and `sort`, including the existing column labels, visibility, type aliases, formatting, widths, alignment, conditional colors, numeric masks, and null-placement overrides.

#### Scenario: Editor validation
- **WHEN** a user configures a YAML language server with the shipped schema
- **THEN** a valid saved view with root identity fields and nested `source` and `view` sections validates without schema errors

#### Scenario: Invalid enum value
- **WHEN** a saved view sets `source.format`, `source.object_mode`, `source.schema_scan`, `view.nulls`, a column `nulls`, a column `type`, or a column `format` to an unsupported value
- **THEN** schema validation reports the field as invalid

#### Scenario: Invalid JSON pointer
- **WHEN** a saved view supplies a `source.json_path` that is not a valid RFC 6901 JSON Pointer
- **THEN** semantic validation records a non-fatal saved-view warning and does not apply the invalid path

#### Scenario: Legacy operation field at the root
- **WHEN** a saved view places `format`, `json_path`, `object_mode`, `table`, `schema_scan`, `limit`, `locale`, `columns`, `filters`, or `sort` at the document root
- **THEN** schema validation rejects the misplaced field and directs the user to `source` or `view`

### Requirement: Saved view validation
The system SHALL validate saved view files structurally and semantically before applying them, including validation of nested source and view configuration.

#### Scenario: Invalid YAML file
- **WHEN** a saved view file contains invalid YAML
- **THEN** the system ignores that view file, records a non-fatal warning, and continues opening the input

#### Scenario: Invalid regex pattern
- **WHEN** a saved view filename pattern or `view.filters` regex is invalid
- **THEN** the system ignores the invalid item, records a non-fatal warning, and continues evaluating other valid configuration

#### Scenario: Invalid source filter
- **WHEN** a saved view contains a `source.filters` predicate unsupported by the selected source
- **THEN** the system reports that source operation as unavailable and does not reinterpret it as a view filter

#### Scenario: Invalid numeric mask
- **WHEN** a number column uses `format: mask` with a mask outside the supported mask grammar
- **THEN** the system ignores the mask for that column, records a non-fatal warning, and falls back to plain display for that column

#### Scenario: Invalid POSIX locale
- **WHEN** a saved view sets an unsupported `view.locale`
- **THEN** the system logs the invalid locale, records a TUI warning, and falls back to `en_US`

#### Scenario: One view per file
- **WHEN** a saved view file is loaded
- **THEN** the system treats the file as exactly one saved view whose canonical name is the file stem

### Requirement: Saved source options
A saved view SHALL apply source-opening and source-query options from `source` before constructing the active source result. Explicit CLI source options SHALL override matching saved values for that invocation.

#### Scenario: Saved JSON starting path
- **WHEN** a matching saved view sets `source.format: json` and `source.json_path: /hits/hits`
- **THEN** source opening selects that embedded JSON value before constructing table columns or rows

#### Scenario: Saved SQLite table
- **WHEN** a matching saved view sets `source.format: sqlite` and `source.table: users`
- **THEN** source opening selects the `users` ordinary table or compatible ordinary view and does not display the table-selection modal

#### Scenario: Saved full schema scan
- **WHEN** a matching saved view sets `source.schema_scan: full`
- **THEN** JSON schema discovery scans all selected rows before the table schema is marked complete

#### Scenario: Saved keyed-object interpretation
- **WHEN** a matching saved view sets `source.format: json` and `source.object_mode: entries`
- **THEN** the selected JSON object's direct members become rows before column configuration is resolved

#### Scenario: Incompatible saved object mode
- **WHEN** a saved view sets an explicit `source.object_mode` for a source shape that cannot interpret an object or map
- **THEN** normal source-option validation reports the incompatibility and does not reinterpret the value as view configuration

#### Scenario: Saved source limit
- **WHEN** a matching saved view sets `source.limit: 2500`
- **THEN** at most 2500 rows are requested for the active source result

#### Scenario: Default source limit
- **WHEN** a SQLite saved view omits `source.limit`
- **THEN** the active SQLite source query uses the default limit of 1000 rows

#### Scenario: CLI source option precedence
- **WHEN** both a saved view and an explicit CLI argument provide the same source option
- **THEN** the explicit CLI value takes precedence for that invocation

### Requirement: Display formatting
The system SHALL apply display formatting from `view` and `view.columns` to rendered cell values without changing raw cell values.

#### Scenario: Plain format
- **WHEN** a column uses `format: plain`
- **THEN** the system renders cell values without display transformation

#### Scenario: Locale number format
- **WHEN** a number column uses `format: locale` and the saved view does not set `view.locale`
- **THEN** the system renders numeric values using the POSIX-style system locale, falling back to `en_US` if locale detection or lookup fails

#### Scenario: View locale override
- **WHEN** a saved view sets `view.locale: en_US` and a number column uses `format: locale`
- **THEN** the system renders locale-formatted values using the saved view locale

#### Scenario: Numeric mask format
- **WHEN** a number column uses `format: mask` and `mask: "0.00"`
- **THEN** the system renders numeric values with two decimal places

#### Scenario: Numeric mask overrides locale
- **WHEN** a saved view sets `view.locale: de_DE` and a number column uses `format: mask` with `mask: "#,##0.00"`
- **THEN** the system renders the value according to the mask grammar rather than substituting locale-specific separators

#### Scenario: String case format
- **WHEN** a string column uses `format: uppercase` or `format: lowercase`
- **THEN** the system renders that column's cell values using the requested case transformation

#### Scenario: Raw and rendered matching
- **WHEN** formatting changes the rendered value for a cell
- **THEN** search and `view.filters` can match either the raw cell value or the rendered cell value

### Requirement: Saved null-placement policy
A saved view SHALL accept `view.nulls: first|last` and `view.columns.<key>.nulls: first|last`, with column configuration overriding the view default and omission using the built-in `last` default.

#### Scenario: View default
- **WHEN** a saved view sets `view.nulls: first`
- **THEN** every view-sorted column without an explicit column policy resolves to nulls first

#### Scenario: Column override
- **WHEN** a saved view sets `view.nulls: first` and `view.columns.deleted_at.nulls: last`
- **THEN** view sorting `deleted_at` places nulls last while other columns inherit nulls first

#### Scenario: Column inherits view policy
- **WHEN** a column omits `nulls`
- **THEN** its configuration retains inheritance so a later view-default change affects it

#### Scenario: Pending structured column policy
- **WHEN** a provisional structured schema has pending canonical column configuration with a `nulls` override
- **THEN** the override is applied when that column is discovered and is used by subsequent view sorting

#### Scenario: Serialize null placement
- **WHEN** the view or a column has an explicit null-placement policy
- **THEN** generated YAML writes it under `view.nulls` or `view.columns.<key>.nulls` and omits it for an inheriting column

### Requirement: Saved view serialization
The system SHALL serialize the current runtime configuration as saved-view YAML conforming to the nested schema, with derived source-query output excluded from persisted configuration.

#### Scenario: Serialize loaded view
- **WHEN** a saved view was loaded from disk and the user opens the view modal
- **THEN** the displayed YAML reflects the current runtime source and view configuration and identifies the loaded saved-view filename

#### Scenario: Serialize new view placeholder
- **WHEN** no saved view was loaded and the user opens the view modal for `foo.bar.csv`
- **THEN** the displayed target filename is `foo.bar.yml` under the saved views directory

#### Scenario: Serialize interactive column changes
- **WHEN** the user changes column widths or visibility before opening the view modal
- **THEN** the YAML includes affected columns under `view.columns`

#### Scenario: Serialize only changed column state
- **WHEN** a column has no saved metadata and no interactive view-state changes
- **THEN** the YAML omits that column from `view.columns`

#### Scenario: Serialize current filename only
- **WHEN** a saved view loaded with multiple filename patterns is displayed in the view modal
- **THEN** the generated YAML includes only the current input filename in root `filenames`

#### Scenario: Serialize default locale omission
- **WHEN** locale formatting uses auto-detected or default behavior
- **THEN** the generated YAML omits `view.locale`

#### Scenario: Serialize placeholder name
- **WHEN** no saved view was loaded for `cat_shards.txt`
- **THEN** the generated YAML includes root `name: cat_shards`

#### Scenario: Serialize layered operations
- **WHEN** source filters, source sort, source limit, view filters, or view sort are active
- **THEN** the generated YAML writes them under their corresponding `source` or `view` section and excludes search state

#### Scenario: Serialize resolved object mode
- **WHEN** an object-capable source resolves automatic or explicit object interpretation to `record` or `entries`
- **THEN** generated YAML writes the resolved value under `source.object_mode`

#### Scenario: Derived SQL is not persisted
- **WHEN** a SQLite source exposes the SQL generated from saved source operations
- **THEN** serialization persists the structured source operations rather than a duplicated generated SQL string

### Requirement: Saved object mode
A saved view SHALL accept `source.object_mode: auto|record|entries` as a format-neutral source-opening option, validate it in the shipped schema and semantic parser, and apply it before an object-capable adapter constructs its table, with explicit CLI values taking precedence.

#### Scenario: Saved entries mode
- **WHEN** a matching saved view sets `source.format: json` and `source.object_mode: entries`
- **THEN** the selected JSON object's direct members become rows before `view.columns` is resolved

#### Scenario: Saved record mode
- **WHEN** a matching saved view sets `source.object_mode: record`
- **THEN** a selected JSON object retains single-row flattened-record interpretation

#### Scenario: Invalid saved mode
- **WHEN** a saved view sets `source.object_mode` to an unsupported value
- **THEN** schema or semantic validation records a non-fatal warning and does not apply that value

#### Scenario: Saved option incompatible with source
- **WHEN** a saved view combines explicit `record` or `entries` mode with a row-stream source or non-object selected shape
- **THEN** source-option validation reports or records the normal incompatibility without treating it as view configuration

#### Scenario: Serialize resolved mode
- **WHEN** a saved view is written for a selected object or map
- **THEN** generated YAML writes the effective `record` or `entries` value under `source.object_mode`

#### Scenario: Saved mode remains authoritative
- **WHEN** `source.object_mode` contains explicit `record` or `entries` and automatic detection changes later
- **THEN** the saved mode remains authoritative unless explicit CLI configuration overrides it

#### Scenario: Omit mode for non-object source
- **WHEN** saved-view YAML is generated for an array, scalar, or row stream
- **THEN** it omits `source.object_mode`

### Requirement: Saved views in non-interactive output
When compiled with saved-view support, batch output SHALL perform the same saved-view selection and apply nested `source` configuration before opening and nested `view` configuration before emitting stdout.

#### Scenario: Automatically selected view
- **WHEN** redirected output opens a filename matching a saved view
- **THEN** its source query and view transform control the bounded output

#### Scenario: Forced named view
- **WHEN** batch output uses `--view <name>`
- **THEN** that named view controls source and view configuration even when its filename patterns do not match

#### Scenario: Saved views disabled
- **WHEN** batch output uses `--no-view`
- **THEN** no saved source table or other saved configuration is applied

#### Scenario: Saved SQLite table selection
- **WHEN** a database has multiple selectable candidates and a matching saved view sets `source.table`
- **THEN** batch output opens that table without interactive selection

#### Scenario: Pending column configuration
- **WHEN** bounded result traversal discovers a column whose `view.columns` configuration was pending
- **THEN** the configuration is applied before final widths and rows are rendered

#### Scenario: View filter produces no rows
- **WHEN** `view.filters` excludes every row from the bounded source result
- **THEN** batch output follows configured header visibility and empty-result rules without refilling

#### Scenario: Interactive transformation starts from saved view
- **WHEN** `--interactive` and `--output <format>` are combined
- **THEN** the TUI starts from nested saved configuration and final output uses subsequent live changes

## ADDED Requirements

### Requirement: Layered saved operations
Saved views SHALL represent source filtering and sorting separately from view filtering and sorting. Source operations SHALL determine the bounded source result before `source.limit`; view operations SHALL transform only that result.

#### Scenario: Source operations precede limit
- **WHEN** a saved SQLite view has source filters, source sort, and `source.limit: 1000`
- **THEN** the generated query applies filtering and sorting before limiting the source result

#### Scenario: View operations follow source limit
- **WHEN** the same saved view also has view filters and view sort
- **THEN** those operations execute locally over at most the rows returned by the source query

#### Scenario: View filter reduces visible rows
- **WHEN** a view filter hides 700 rows from a 1000-row source result
- **THEN** 300 rows remain visible and the system does not fetch replacement rows

#### Scenario: Search is transient
- **WHEN** search is active while a saved view is serialized
- **THEN** search remains a transient navigation operation and is omitted from both sections

### Requirement: Relational saved-view column matching
The system SHALL project relational column source identities into deterministic keys under `view.columns` without using display labels as runtime identity.

#### Scenario: Unique SQLite column name
- **WHEN** a selected table contains exactly one column named `email` and `view.columns.email` is configured
- **THEN** that configuration applies to the column with the matching relational source identity

#### Scenario: Duplicate SQLite column name
- **WHEN** a selected table contains duplicate `name` columns and `view.columns.name#2` is configured
- **THEN** that configuration applies to the second occurrence in source order

#### Scenario: Ambiguous unsuffixed SQLite name
- **WHEN** `view.columns.name` is configured and the selected table contains multiple columns with that name
- **THEN** the system does not guess and records a non-fatal warning recommending a deterministic occurrence key

#### Scenario: Serialize selected table
- **WHEN** the current view is opened from a SQLite table and saved-view YAML is generated
- **THEN** the YAML includes `source.format: sqlite`, `source.table`, and canonical relational keys under `view.columns`
