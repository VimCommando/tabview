## ADDED Requirements

### Requirement: Saved view CLI overrides
When compiled with the `saved-views` feature, the Rust executable SHALL accept saved view override arguments that force a named saved view or disable saved view application for the current invocation.

#### Scenario: Force saved view
- **WHEN** a user runs `tabview --view cat-shards sample/data.csv`
- **THEN** the command is accepted and saved view selection uses the saved view named `cat-shards`

#### Scenario: Force saved view with extension
- **WHEN** a user runs `tabview --view cat-shards.yml sample/data.csv`
- **THEN** the command is accepted and saved view selection uses the saved view named `cat-shards`

#### Scenario: Disable saved views
- **WHEN** a user runs `tabview --no-view sample/data.csv`
- **THEN** the command is accepted and saved view discovery and application are skipped

#### Scenario: Conflicting saved view flags
- **WHEN** a user runs `tabview --view cat-shards --no-view sample/data.csv`
- **THEN** argument parsing rejects the invocation with a clear error

#### Scenario: Saved views feature disabled
- **WHEN** the binary is compiled without the `saved-views` feature
- **THEN** the saved view override arguments are not part of the supported command-line surface
