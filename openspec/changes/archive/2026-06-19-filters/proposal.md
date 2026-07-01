## Why

Users need a fast way to reduce visible rows while inspecting large or noisy tabular data without leaving the terminal viewer. Filtering on the current column matches the existing column-centric sort and navigation model and keeps the interaction compact.

## What Changes

- Add filter-in and filter-out commands bound to `f` and `F`.
- Open a modal filter prompt that accepts a condition for the current column and shows radio-style choices for text, regex, and numeric filters.
- Use the current column type as the numeric filter hint: numeric columns default to numeric filters, while non-numeric text columns disable the numeric choice.
- Let users cycle enabled filter types with `Tab` while keeping the condition input focused.
- Apply filters to data rows only; header rows remain visible and are never removed by filtering.
- Mark filtered columns in the header with an indicator character.
- Support text substring matching, standard regular expressions, and numeric comparisons using `<`, `>`, `<=`, `>=`, and `=` with recognized numeric suffixes.
- Keep filtering as a viewer operation that changes row visibility, not underlying table data.

## Capabilities

### New Capabilities
- `filters`: Defines filter-in and filter-out behavior, condition parsing, row visibility rules, header indicators, and interaction with current-column table viewing.

### Modified Capabilities
- `rust-architecture`: Update feature exclusions so filtering is allowed as a viewer row-visibility operation while editing, formulas, and persistent data mutation remain excluded.

## Impact

- Affects TUI keybinding registry, modal prompt handling, table view state, row visibility mapping, header rendering, search/navigation behavior over visible rows, and render-level tests.
- May reuse or extend existing column type classification plus numeric parsing and suffix handling from numeric sort to keep comparison semantics consistent.
