## ADDED Requirements

### Requirement: Saved views in non-interactive output
When compiled with saved-view support, table output SHALL perform the same automatic or forced saved-view selection and apply the same source options, column configuration, display formatting, visibility, alignment, widths, null placement, sort, and filters as the interactive viewer before emitting stdout.

#### Scenario: Automatically selected view
- **WHEN** redirected output opens a filename matching a saved view and saved views are not disabled
- **THEN** the matching view controls the non-interactive table

#### Scenario: Forced named view
- **WHEN** table output uses `--view <name>`
- **THEN** that named view controls output even when its filename patterns do not match the input

#### Scenario: Saved views disabled
- **WHEN** table output uses `--no-view`
- **THEN** no saved view is discovered or applied and default table presentation is emitted

#### Scenario: Pending structured column configuration
- **WHEN** complete table traversal discovers a structured column whose saved configuration was pending under a provisional schema
- **THEN** the configuration is applied before final widths and output rows are rendered

#### Scenario: Saved filter produces no rows
- **WHEN** a saved view filter excludes every source row
- **THEN** non-interactive output follows the configured header visibility and empty-result rules

#### Scenario: Interactive transformation starts from saved view
- **WHEN** `--interactive` and `--output <format>` are combined for an input with an automatically or explicitly selected saved view
- **THEN** the TUI starts from that saved configuration and final output uses the resulting live state, including any further interactive changes
