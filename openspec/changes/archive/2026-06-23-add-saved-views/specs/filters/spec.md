## MODIFIED Requirements

### Requirement: Text filter conditions
The system SHALL apply a text filter when the prompt's selected type is text, and SHALL match data rows whose current-column raw value or saved-view-rendered value contains the entered text as a substring.

#### Scenario: Text filter-in keeps substring matches
- **WHEN** a user applies filter-in on a column with condition `foo`
- **THEN** rows with raw or rendered values `foobar`, `barfoo`, `foo`, and `barfoobaz` in that column remain visible

#### Scenario: Text filter-out hides substring matches
- **WHEN** a user applies filter-out on a column with condition `foo`
- **THEN** rows with raw or rendered values containing `foo` in that column are hidden and rows without that substring in either representation remain visible

#### Scenario: Text filter matches formatted value
- **WHEN** a saved view renders raw value `1000` as `1,000` and the user applies text filter-in with condition `1,000`
- **THEN** that row remains visible

### Requirement: Regex filter conditions
The system SHALL apply a regex filter when the prompt's selected type is regex, and SHALL match data rows by applying standard regular expression semantics to the current-column raw value and saved-view-rendered value.

#### Scenario: Regex filter-in keeps regex matches
- **WHEN** a user applies filter-in on a column with condition `^foo[0-9]+$`
- **THEN** rows with current-column raw or rendered values such as `foo1` and `foo20` remain visible and rows such as `xfoo1` or `foo` are hidden

#### Scenario: Invalid regex is not applied
- **WHEN** a user submits a regex filter condition that cannot compile
- **THEN** the viewer keeps the filter prompt open, reports the condition error non-fatally, and leaves active filters unchanged

#### Scenario: Regex filter matches formatted value
- **WHEN** a saved view renders raw value `1000` as `1,000` and the user applies regex filter-in with condition `^1,`
- **THEN** that row remains visible

## ADDED Requirements

### Requirement: Filter persistence in saved views
The system SHALL include active filters when serializing the current runtime view configuration to saved view YAML.

#### Scenario: Persist active filter
- **WHEN** a text, regex, or numeric filter is active and the user opens the saved view modal
- **THEN** the generated YAML includes the filter's source column, action, kind, and condition

#### Scenario: Restore saved filter
- **WHEN** a saved view file contains a filter whose source column exists in the loaded table
- **THEN** the system applies that filter after loading the table and resolving columns
