# filters Specification

## Purpose
TBD - created by archiving change filters. Update Purpose after archive.
## Requirements
### Requirement: Filter command interaction
The system SHALL bind `f` to filter-in and `F` to filter-out for the current column and SHALL open a modal prompt to accept the filter condition.

#### Scenario: Open filter-in prompt
- **WHEN** a user presses `f` while focused on a data cell in column 2
- **THEN** the viewer opens a filter-in prompt for column 2

#### Scenario: Open filter-out prompt
- **WHEN** a user presses `F` while focused on a data cell in column 2
- **THEN** the viewer opens a filter-out prompt for column 2

#### Scenario: Cancel filter prompt
- **WHEN** a filter prompt is open and the user presses `Esc`
- **THEN** the prompt closes without changing active filters or row visibility

#### Scenario: Clear current column filters
- **WHEN** a filter prompt is submitted with an empty condition
- **THEN** active filters for the current column are removed and row visibility is recalculated

### Requirement: Filter type selector
The filter prompt SHALL show radio-style choices for text, regex, and numeric filter types, SHALL keep keyboard focus on the condition input, and SHALL cycle the selected enabled filter type when the user presses `Tab`.

#### Scenario: Text column defaults to text with numeric disabled
- **WHEN** a user opens a filter prompt on a column that is not classified as numeric
- **THEN** text is the selected filter type and numeric is disabled

#### Scenario: Numeric column defaults to numeric
- **WHEN** a user opens a filter prompt on a column classified as numeric
- **THEN** numeric is the selected filter type and text, regex, and numeric are enabled

#### Scenario: Tab cycles enabled filter types
- **WHEN** a filter prompt is open and the user presses `Tab`
- **THEN** the selected filter type changes to the next enabled radio choice and keyboard focus remains on the condition input

#### Scenario: Text input remains focused
- **WHEN** a filter prompt is open and the user types printable characters after cycling the filter type
- **THEN** the characters are appended to the condition input rather than moving focus away from the input

### Requirement: Text filter conditions
The system SHALL apply a text filter when the prompt's selected type is text, and SHALL match data rows whose current-column value contains the entered text as a substring.

#### Scenario: Text filter-in keeps substring matches
- **WHEN** a user applies filter-in on a column with condition `foo`
- **THEN** rows with values `foobar`, `barfoo`, `foo`, and `barfoobaz` in that column remain visible

#### Scenario: Text filter-out hides substring matches
- **WHEN** a user applies filter-out on a column with condition `foo`
- **THEN** rows with values containing `foo` in that column are hidden and rows without that substring remain visible

### Requirement: Regex filter conditions
The system SHALL apply a regex filter when the prompt's selected type is regex, and SHALL match data rows using standard regular expression semantics.

#### Scenario: Regex filter-in keeps regex matches
- **WHEN** a user applies filter-in on a column with condition `^foo[0-9]+$`
- **THEN** rows with current-column values such as `foo1` and `foo20` remain visible and rows such as `xfoo1` or `foo` are hidden

#### Scenario: Invalid regex is not applied
- **WHEN** a user submits a regex filter condition that cannot compile
- **THEN** the viewer keeps the filter prompt open, reports the condition error non-fatally, and leaves active filters unchanged

### Requirement: Numeric filter conditions
The system SHALL allow numeric filters only on columns classified as numeric, SHALL apply a numeric filter when the prompt's selected type is numeric, and SHALL compare current-column values using numeric magnitude with recognized suffixes.

#### Scenario: Numeric unavailable on text column
- **WHEN** a user opens a filter prompt on a column that is not classified as numeric
- **THEN** the numeric filter type cannot be selected or submitted

#### Scenario: Numeric less-than filter
- **WHEN** a user applies filter-in on a numeric column with numeric condition `<10`
- **THEN** rows with numeric current-column values less than 10 remain visible and rows with values greater than or equal to 10 are hidden

#### Scenario: Numeric greater-than-or-equal filter
- **WHEN** a user applies filter-in on a numeric column with numeric condition `>=20`
- **THEN** rows with numeric current-column values greater than or equal to 20 remain visible and rows with values less than 20 are hidden

#### Scenario: Byte suffix comparison
- **WHEN** a user applies filter-in on a numeric column with numeric condition `<2gb`
- **THEN** rows with suffixed numeric values whose byte magnitude is less than 2 gigabytes remain visible

#### Scenario: Text search remains available on numeric column
- **WHEN** a user opens a filter prompt on a numeric column and selects text with condition `gb`
- **THEN** rows with current-column values containing `gb` match the text condition

#### Scenario: Non-numeric value in numeric filter
- **WHEN** a numeric filter is active for a column
- **THEN** rows whose current-column value cannot be parsed as a numeric value do not match the numeric condition

### Requirement: Header and filtered column indicators
The system SHALL never filter out header rows and SHALL render an indicator character on each visible header cell whose column has an active filter.

#### Scenario: Header remains visible after filtering
- **WHEN** a table has a visible header row and a filter hides every data row
- **THEN** the header row remains visible

#### Scenario: Filtered header column is marked
- **WHEN** a visible header column has one or more active filters
- **THEN** the rendered header cell for that column includes a filter indicator character

### Requirement: Filtered row visibility
The system SHALL treat filtering as row visibility state and MUST NOT mutate the underlying parsed cell data.

#### Scenario: Filter-in row visibility
- **WHEN** a filter-in condition is active for the current column
- **THEN** only data rows whose column value matches the condition are visible

#### Scenario: Filter-out row visibility
- **WHEN** a filter-out condition is active for the current column
- **THEN** only data rows whose column value does not match the condition are visible

#### Scenario: Multiple active filters
- **WHEN** filters are active on multiple columns
- **THEN** a data row is visible only when it satisfies every active filter

#### Scenario: Operations use visible rows
- **WHEN** filters are active
- **THEN** cursor movement, search traversal, skip-to-change, cell popup, and yank operate over visible data rows

#### Scenario: Reload reapplies filters
- **WHEN** a user reloads a file while filters are active
- **THEN** the viewer reapplies active filters to the reloaded data where possible and keeps the cursor within the visible row range

