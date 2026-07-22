## ADDED Requirements

### Requirement: Saved object mode
A saved view SHALL accept top-level `object_mode: auto|record|entries` as a format-neutral source-opening option, validate it in the shipped schema and semantic parser, and apply it before an object-capable adapter constructs its table, with explicit CLI values taking precedence.

#### Scenario: Saved entries mode
- **WHEN** a matching saved view sets `format: json` and `object_mode: entries`
- **THEN** the selected JSON object's direct members become rows before column configuration is resolved

#### Scenario: Saved record mode
- **WHEN** a matching saved view sets `object_mode: record`
- **THEN** a selected JSON object retains single-row flattened-record interpretation

#### Scenario: Invalid saved mode
- **WHEN** a saved view sets `object_mode` to an unsupported value
- **THEN** schema or semantic validation records a non-fatal saved-view warning and does not apply that value

#### Scenario: Saved option incompatible with format
- **WHEN** a saved view combines explicit `record` or `entries` mode with a row-stream format that has no selected object/map, such as delimited input or NDJSON
- **THEN** source-option validation records a clear warning and does not apply the incompatible combination

#### Scenario: Saved option incompatible with selected shape
- **WHEN** a saved view supplies explicit `record` or `entries` but the structured adapter selects an array or scalar
- **THEN** source-option validation records a clear warning and does not apply the incompatible value

#### Scenario: Serialize resolved automatic mode
- **WHEN** a saved view is written for a selected object/map whose requested mode is `auto`
- **THEN** generated saved-view YAML includes `object_mode` with the resolved `record` or `entries` value

#### Scenario: Serialize explicit mode
- **WHEN** a saved view is written for a selected object/map whose effective mode is explicitly `record` or `entries`
- **THEN** generated saved-view YAML includes `object_mode` with that value

#### Scenario: Saved mode is stable across detector changes
- **WHEN** a saved view contains explicit `record` or `entries` mode and the default automatic detector changes in a later version
- **THEN** the saved mode remains authoritative unless an explicit CLI value overrides it

#### Scenario: Omit mode for a non-object shape
- **WHEN** a saved view is written while the selected value is an array or scalar, or the source is a row stream
- **THEN** generated saved-view YAML omits `object_mode`
