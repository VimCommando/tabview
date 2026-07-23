## ADDED Requirements

### Requirement: Format-neutral object mode option
The Rust executable SHALL accept `--object-mode auto|record|entries` and use `auto` when omitted. The shared CLI and source-option names SHALL be independent of any one serialization format so object-capable adapters, including future YAML and TOON adapters, can reuse them. After format resolution and structured-value selection, an adapter SHALL apply the mode only to a selected object/map and SHALL reject explicit incompatible formats or selected shapes clearly. This option SHALL NOT alter stdin buffering or imply an input format.

#### Scenario: Force keyed entries
- **WHEN** a user runs `tabview --format json --object-mode entries repositories.json`
- **THEN** the selected JSON object's direct members become table rows without automatic shape inference

#### Scenario: Preserve record behavior
- **WHEN** a user runs `tabview --format json --object-mode record object.json`
- **THEN** the selected object is represented as one flattened row

#### Scenario: Default automatic mode
- **WHEN** a user opens an object-capable structured input without supplying `--object-mode`
- **THEN** the effective object mode is `auto`

#### Scenario: Incompatible delimited format
- **WHEN** a user combines an explicit non-default object mode with `--format delimited`
- **THEN** argument or source-option validation rejects the combination with a clear error

#### Scenario: Incompatible NDJSON format
- **WHEN** a user combines `--format ndjson` with explicit `record` or `entries` object mode
- **THEN** argument or source-option validation rejects the combination because NDJSON retains one row per logical document

#### Scenario: Incompatible selected shape
- **WHEN** an object-capable adapter selects an array or scalar and the CLI explicitly requests `record` or `entries`
- **THEN** source opening fails with a clear incompatible-shape error

#### Scenario: Object mode does not imply stdin format
- **WHEN** stdin format remains unresolved while `--object-mode` is supplied
- **THEN** this option does not change the stdin buffering or format-resolution policy owned by the non-interactive input workflow

#### Scenario: CLI overrides saved mode
- **WHEN** a saved view selects one object mode and the user supplies a different `--object-mode`
- **THEN** the explicit CLI value takes precedence for that invocation
