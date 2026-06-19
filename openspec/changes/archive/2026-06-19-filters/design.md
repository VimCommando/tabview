## Context

The current Rust viewer keeps header data separately from data rows in `TableView`, exposes row-oriented operations through `view.rows()`, and implements search, sort, skip-to-change, rendering, and yanking over that visible row slice. The command registry is centralized in `src/command/mod.rs`, modal popups are modeled by `ui::Popup`, and search input already demonstrates prompt-style modal editing in `App`.

Filtering crosses command handling, modal input, view state, rendering, and table operations. It also conflicts with the existing `rust-architecture` feature exclusion that explicitly disallows filtering, so this change treats filtering as an allowed row-visibility operation rather than a data mutation feature.

## Goals / Non-Goals

**Goals:**
- Add `f` filter-in and `F` filter-out keybindings for the current column.
- Prompt for a filter condition in a modal with radio-style text, regex, and numeric type choices, then apply it without mutating source cell data.
- Default to numeric filters for numeric columns, disable numeric filters for non-numeric columns, and allow text or regex filters on every column.
- Keep the condition input focused while `Tab` cycles the selected enabled filter type.
- Keep header rows visible and show an indicator on filtered header columns.
- Make navigation, search, skip, yanking, and rendering operate over visible data rows.
- Reuse numeric suffix semantics from numeric sort for numeric filter comparisons.

**Non-Goals:**
- No CLI filter arguments.
- No editing, formulas, projection, or persisted data transformation.
- No separate filter language beyond text substrings, standard regex, and simple numeric comparisons.
- No new large-file execution model in this change.

## Decisions

### Keep all data rows and add a visible-row mapping

`TableView` should retain all data rows and maintain active filters plus a derived `visible_rows: Vec<usize>` mapping from visible row index to source row index. Cursor and viewport coordinates should continue to be visible-row coordinates so existing UI concepts remain stable. Rendering and operations that need cell values should access rows through visible-row helpers instead of reading the backing vector directly.

Alternative considered: physically remove filtered-out rows from `rows`. That is simpler initially, but it makes clearing filters, reload preservation, sorting, yanking, and future large-file work harder because the original row set is lost or must be rebuilt elsewhere.

### Represent filters as explicitly selected typed conditions

Add `src/ops/filter.rs` with a `FilterKind` enum for text, regex, and numeric, a `FilterCondition` enum, and a `FilterMode` enum for filter-in versus filter-out. The modal owns the selected `FilterKind`; parsing validates the current input according to that selected kind rather than inferring kind from the input. Text filters treat the input literally as a substring. Regex filters compile the input as a regular expression. Numeric filters parse an operator and numeric operand.

Alternative considered: inferring type from punctuation and operators. The refined interaction calls for explicit radio choices, which avoids ambiguity between literal text, regex metacharacters, and numeric-looking text.

### Gate numeric filters using column metadata

Use the existing column metadata that classifies numeric columns as the hint for numeric filter availability. When the current column is numeric, the modal should default to numeric and keep text, regex, and numeric enabled because users may still need to text-search suffixes or other formatted values. When the current column is not numeric, the modal should default to text and disable numeric; `Tab` should skip disabled numeric while cycling filter type.

Alternative considered: allow numeric on every column and reject later if parsing fails. That produces a worse prompt experience and ignores the column type signal already computed by the table model.

### Keep condition input focused while type selection changes

The filter modal should render a radio-style selector, but keyboard focus should remain with the condition input. Printable keys update the input, editing keys edit the input, and `Tab` cycles the selected enabled filter type. This preserves fast typing while still allowing explicit type changes.

Alternative considered: move focus between radio controls and input fields. That would make the terminal workflow slower and require extra focus-management states for little benefit.

### Add `regex` as a dependency

Use the standard Rust `regex` crate for regular expression filters. Invalid regex input should keep the modal open and show a non-fatal error message rather than applying a broken filter.

Alternative considered: hand-rolled pattern matching or shell-style globs. That would not satisfy standard regex behavior and would create edge cases users do not expect.

### Reuse numeric sort suffix parsing

Numeric filters should share suffix parsing and column profile behavior with numeric sort. The implementation should refactor private numeric parsing in `src/ops/sort.rs` into reusable helpers, or move shared numeric parsing into a dedicated module used by both sort and filter.

Alternative considered: a separate numeric parser for filters. That risks inconsistent handling for byte suffixes, percent values, time-context `m`, placeholders, and other numeric edge cases already covered by sort tests.

### Treat empty filter submissions as clear-current-column

Submitting an empty condition from either filter modal should remove active filters for the current column. `Esc` should cancel without changing filters. This gives users a minimal clearing path without introducing another keybinding in the initial change.

Alternative considered: add a dedicated clear-filter command. That may still be useful later, but it is not required by the requested `f/F` binding surface.

## Risks / Trade-offs

- Visible-row coordinates can diverge from source-row coordinates -> Keep cursor, viewport, and key commands in visible coordinates and isolate source-row lookup inside `TableView`.
- Existing helpers accept `&[Vec<String>]` and may accidentally bypass filters -> Introduce explicit visible-row accessors and update call sites rather than exposing the backing rows as the default operation input.
- Type selection can be easy to miss in a compact terminal modal -> Render the selected type clearly, show disabled numeric distinctly on non-numeric columns, and test `Tab` cycling behavior.
- Users may want text searches on numeric columns with suffixes -> Keep text and regex enabled even when numeric is the default.
- Numeric suffix behavior can drift from sort behavior -> Share parser/profile code and add filter tests for byte suffixes and time-context suffixes.
- Header indicators can affect column width calculations -> Include indicator width when computing header display text so layout remains stable and columns do not overlap.
