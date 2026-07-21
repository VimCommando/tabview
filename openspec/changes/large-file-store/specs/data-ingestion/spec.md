## MODIFIED Requirements

### Requirement: Large-file groundwork
The system SHALL use the previously introduced lazy threshold and store abstractions in the live format-aware table-opening path rather than leaving them as an unused prototype.

#### Scenario: Lazy threshold is centralized
- **WHEN** a size-based format adapter decides whether a seekable file requires incremental handling
- **THEN** the default lazy threshold is available as a named configurable constant set to 100 MiB

#### Scenario: Live viewer uses table store
- **WHEN** any supported source is opened for the interactive viewer
- **THEN** row access is routed through the selected in-memory or incremental table store

#### Scenario: Existing delimited compatibility remains
- **WHEN** an existing CSV-like input is opened with encoding, delimiter, quote, or quoting options
- **THEN** format-aware opening preserves the established decoding, parsing, normalization, and header-classification behavior
