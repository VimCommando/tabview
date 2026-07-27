## 1. Draft Model and Validation

- [ ] 1.1 Add a modal-owned View Configuration draft that snapshots the active source generation, ordered filters and sorts, null policies, and supported column presentation.
- [ ] 1.2 Add canonical draft validation for predicates, values, stable column identities, duplicate entries, null policies, and presentation fields.
- [ ] 1.3 Add an atomic application boundary that commits transform and presentation state together while preserving the previous active view on failure.

## 2. Editor Interaction

- [ ] 2.1 Replace the View Configuration summary popup with keyboard-navigable Filters, Sort, Nulls, and Columns sections.
- [ ] 2.2 Add filter item workflows for selecting a column and adding, editing, removing, and reordering predicates.
- [ ] 2.3 Add sort item workflows for selecting a column and adding, editing, removing, and reordering mode, direction, null behavior, and precedence.
- [ ] 2.4 Add view-wide and per-column null-placement controls.
- [ ] 2.5 Add controls for the supported column visibility, label, format, width, and display-order presentation fields.
- [ ] 2.6 Add Apply, Cancel, focused validation feedback, scrolling, and small-terminal fallback behavior.

## 3. Shared State and Execution

- [ ] 3.1 Initialize every editor draft from the authoritative active state so Column Info and quick-command changes are reflected on open.
- [ ] 3.2 Ensure an applied draft is reflected by Column Info, quick commands, rendering, and saved-view serialization without duplicate operation state.
- [ ] 3.3 Route local view recomputation through the standard Tokio runtime with progress and cancellation while preventing source-query replacement.
- [ ] 3.4 Invalidate or reject drafts whose source generation or stable column identities become stale.

## 4. Verification and Documentation

- [ ] 4.1 Add unit tests for draft construction, stable identity, validation, ordering, atomic commit, cancellation, and failure preservation.
- [ ] 4.2 Add interaction tests for section navigation and editing filters, sorts, nulls, and column presentation.
- [ ] 4.3 Add synchronization tests covering Column Info, quick commands, the global editor, rendering, and saved-view serialization.
- [ ] 4.4 Add regressions proving View Configuration behaves consistently across source types and never reopens, expands, or re-queries the source.
- [ ] 4.5 Add render tests for validation errors, scrolling, empty sections, ordered summaries, and small-terminal fallback.
- [ ] 4.6 Update keybinding help and user documentation for the expanded View Configuration workflow.
- [ ] 4.7 Run formatting, Clippy, default-feature tests, all-feature tests, and supported platform checks.
