## 1. Filter Condition Core

- [x] 1.1 Add the `regex` crate dependency and expose it through normal Cargo builds.
- [x] 1.2 Refactor shared numeric parsing/comparison from `src/ops/sort.rs` so filters can reuse suffix-aware numeric magnitude logic.
- [x] 1.3 Add `src/ops/filter.rs` with filter mode, filter kind, comparison operator, typed condition, parse errors, and selected-kind parsing.
- [x] 1.4 Implement text substring matching, regex matching, and numeric comparison matching against a cell value and column numeric profile.
- [x] 1.5 Add unit tests for selected text filters, selected regex filters, invalid regex handling, numeric operators, byte suffix comparisons, numeric gating, and non-numeric numeric-filter cells.

## 2. View State and Row Visibility

- [x] 2.1 Extend `TableView` to keep all backing rows plus active filters and a visible-row index mapping.
- [x] 2.2 Add visible-row accessors for current cell, visible rows, visible row count, and source-row lookup without exposing filtered-out rows to normal operations.
- [x] 2.3 Add APIs to apply filter-in, apply filter-out, clear filters for a column, report filtered columns, and recompute row visibility.
- [x] 2.4 Keep cursor, viewport, mark, and goto behavior clamped to visible rows after filters are applied or cleared.
- [x] 2.5 Update sorting to preserve active filters and recompute visible rows after backing rows are reordered.

## 3. Command and Modal Flow

- [x] 3.1 Add `FilterIn` and `FilterOut` commands bound to `f` and `F` in the command registry and dynamic help.
- [x] 3.2 Add filter prompt popup state that records mode, target column, selected filter kind, enabled filter kinds, current input, and any validation error.
- [x] 3.3 Default the prompt to numeric for numeric columns, default to text for non-numeric columns, and disable numeric on non-numeric columns.
- [x] 3.4 Implement filter prompt editing, `Tab` kind cycling with input focus retained, `Esc` cancellation, `Enter` apply, and empty-submit clear-current-column behavior.
- [x] 3.5 Update app operations so search, skip-to-change, yanking, cell popup, info text, and reload use visible rows and preserve/reapply filters.
- [x] 3.6 Add command and app-level tests for keybindings, prompt defaults, `Tab` cycling, prompt cancellation, prompt application, and clearing filters.

## 4. Rendering

- [x] 4.1 Render filter prompts with the target column, radio-style filter type choices, disabled numeric state, and current condition text.
- [x] 4.2 Render non-fatal condition errors in the filter prompt when regex or numeric parsing fails.
- [x] 4.3 Add a header indicator character to visible header cells for columns with active filters.
- [x] 4.4 Include header indicators in width and truncation handling so filtered headers do not overlap adjacent cells.
- [x] 4.5 Add Ratatui buffer tests for filtered row rendering, empty filtered result with header visible, and filtered header indicators.

## 5. Integration and Documentation

- [x] 5.1 Add view-level tests for filter-in, filter-out, multiple active filters, header preservation, and backing row immutability.
- [x] 5.2 Add reload tests proving active filters are reapplied and cursor position remains valid.
- [x] 5.3 Update README or help-oriented documentation to describe `f` and `F` filter behavior and empty-submit clearing.
- [x] 5.4 Run `cargo fmt` and `cargo test` and fix any regressions.
