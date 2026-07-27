## Why

The SQLite work established separate source and view operation layers, but the runtime View Configuration modal currently exposes only a summary and a few current-column actions. A complete source-neutral editor for filters, sort precedence, null behavior, and column presentation needs focused interaction design and should not block archiving the SQLite data-source change.

## What Changes

- Expand the runtime View Configuration modal from a summary into a complete editor for global view filters, ordered sort precedence, null placement, and column presentation.
- Keep all edits source-neutral: applying view changes recomputes the local result without reopening or re-querying the source.
- Synchronize the global editor with Column Info and existing quick filter/sort commands through the shared `ViewTransform` state.
- Preserve the separation between runtime View Configuration, Source Configuration, and the Saved View YAML modal.
- Add interaction and rendering coverage for editing, validation, navigation, synchronization, apply/cancel behavior, and failure preservation.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `table-operations`: Promote View Configuration into a complete global view editor while retaining source-neutral execution and shared state with current-column controls.

## Impact

- Affected code: runtime modal state and input handling in `src/lib.rs`, modal rendering in `src/ui`, view-transform editing in `src/view`, and related command and interaction tests.
- Existing source adapters, source-query behavior, saved-view schema, and dependency features are unchanged.
- The work is a focused UI follow-up to `add-turso-sqlite-support`.
