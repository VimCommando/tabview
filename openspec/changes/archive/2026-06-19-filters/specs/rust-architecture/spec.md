## MODIFIED Requirements

### Requirement: Feature exclusions
The implementation SHALL NOT add editing, formulas, or persistent data mutation features beyond current viewer operations. Filtering SHALL be allowed only as a viewer row-visibility operation that does not mutate parsed cell data.

#### Scenario: User attempts to edit a cell
- **WHEN** a user presses ordinary printable keys outside of search or filter entry
- **THEN** the viewer does not modify table cell contents
