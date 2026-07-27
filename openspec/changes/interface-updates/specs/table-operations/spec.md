## ADDED Requirements

### Requirement: Global View Configuration editor
The interactive runtime SHALL provide a source-neutral View Configuration editor for the complete local view state. The editor SHALL use stable column identity, stage edits until Apply, and share its active state with Column Info, quick commands, rendering, and saved-view serialization.

#### Scenario: Open the global editor
- **WHEN** the user opens View Configuration for any supported source
- **THEN** the editor shows the active ordered filters, sort precedence, view-wide and per-column null behavior, and supported column presentation

#### Scenario: Edit filters across columns
- **WHEN** the user adds, edits, removes, or reorders view filters in the editor
- **THEN** the draft preserves each predicate's stable column identity, mode, comparison behavior, and list order

#### Scenario: Edit sort precedence
- **WHEN** the user adds, edits, removes, or reorders view sort keys in the editor
- **THEN** the draft displays and preserves the exact multi-column precedence, direction, mode, and resolved null behavior

#### Scenario: Edit null behavior and presentation
- **WHEN** the user changes the view-wide null default, a per-column null override, or a supported column presentation field
- **THEN** the change is recorded in the same draft as filter and sort edits

#### Scenario: Apply a valid draft
- **WHEN** the user applies a complete valid View Configuration draft
- **THEN** the viewer atomically recomputes and presents the local view over the fixed active source result without reopening, expanding, or re-querying the source

#### Scenario: Cancel a draft
- **WHEN** the user cancels or escapes from View Configuration after editing the draft
- **THEN** the active view, cursor, viewport, source result, and saved state remain unchanged

#### Scenario: Invalid draft
- **WHEN** the draft contains an invalid predicate, incompatible value, duplicate or stale column identity, or another invalid field
- **THEN** Apply is rejected with focused validation feedback and the previously active view remains unchanged

#### Scenario: Local recomputation fails
- **WHEN** validation succeeds but indexing, materialization, or local execution fails
- **THEN** the failure is reported and the previously active view, cursor, viewport, and source result remain active

#### Scenario: Column Info changes are visible
- **WHEN** the user changes a current-column view filter, sort, null override, or presentation setting through Column Info and later opens View Configuration
- **THEN** the editor draft reflects the current active state rather than a separate copy

#### Scenario: Editor changes are visible contextually
- **WHEN** the user applies an edit for a column and later opens Column Info for that column
- **THEN** Column Info reflects the applied filter, sort, null, and presentation state

#### Scenario: Quick commands remain view-scoped
- **WHEN** the user invokes an existing quick filter or sort command before or after using the editor
- **THEN** it changes the same active local view state and never implicitly starts a source query

#### Scenario: Source generation changes
- **WHEN** reload or source replacement changes the source generation while a View Configuration draft exists
- **THEN** the stale draft cannot be applied to the new source generation or its same-position columns

#### Scenario: Runtime and persistence modals remain distinct
- **WHEN** the user opens Source Configuration or the Saved View YAML modal
- **THEN** neither workflow is replaced by the global View Configuration editor
