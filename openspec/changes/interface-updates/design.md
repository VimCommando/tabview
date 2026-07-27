## Context

`add-turso-sqlite-support` separated source operations from source-neutral view operations and added a lightweight View Configuration modal. The current modal summarizes the active `ViewTransform`, can clear filters and sorts, toggles null placement, and links to Column Info. Editing individual filters, ordered multi-column sorts, and the broader presentation model still depends on contextual commands or saved-view YAML.

The follow-up crosses application modal state, rendering, view-transform execution, presentation state, and interaction tests. It must work identically for delimited, JSON, NDJSON, and SQLite sources, and it must preserve the existing rule that view changes never reopen or re-query a source.

## Goals / Non-Goals

**Goals:**

- Provide one keyboard-navigable runtime editor for the complete local view configuration.
- Edit ordered filters and sort keys, view-wide and per-column null placement, and column presentation.
- Keep the editor, Column Info, and quick commands synchronized through one authoritative runtime state.
- Validate and apply a complete edit atomically while retaining the last valid result on failure.
- Clearly distinguish runtime View Configuration from Source Configuration and Saved View YAML.

**Non-Goals:**

- Changing source filters, source sorts, limits, query provenance, or adapter capabilities.
- Changing the saved-view YAML schema or adding a second persistence format.
- Adding SQLite, Turso, or other source-specific behavior.
- Redesigning the table body, footer, picker, Source Configuration, or Saved View modal beyond consistency needed for the editor.
- Adding mouse-first interaction.

## Decisions

### Use a staged runtime draft

Opening View Configuration snapshots the active local transform and presentation state into a modal-owned draft. Navigation and field edits affect only that draft. Apply validates the complete draft and atomically replaces the active view; Cancel or Escape discards it.

This is preferable to applying each keystroke immediately because multi-field predicates and sort entries have temporarily incomplete states, and a failed local recomputation must not leave the table partially updated. The summary modal's direct shortcuts remain available until the editor replaces them.

### Keep one authoritative active view

The editor will translate its validated draft into the same runtime `ViewTransform` and presentation state used by Column Info, quick commands, rendering, and saved-view serialization. It will not create an editor-specific operation store.

When the modal opens, it always snapshots current active state, including changes previously made through Column Info or quick commands. After Apply, subsequently opened contextual controls read the newly active state. This avoids synchronization events and stale duplicated models.

### Model the modal as sections with ordered collections

The modal will expose four sections:

1. Filters: add, edit, remove, and reorder predicates across columns.
2. Sort: add, edit, remove, and reorder keys, with list order defining precedence.
3. Nulls: edit the view-wide default and optional per-column overrides.
4. Columns: edit supported runtime presentation fields such as visibility, label, format, width behavior, and display order.

Each collection item uses stable column identity internally and renders the current display label. Missing or stale identities are validation errors rather than silently targeting a same-position column.

A sectioned editor is preferable to a raw YAML text area because it can constrain values, preserve stable identities, provide focused validation, and remain usable when saved views are not compiled in.

### Reuse canonical validation and atomic execution

Apply uses the same predicate, comparator, null-policy, and column-configuration validation as existing commands and saved-view loading. The active view and table remain unchanged until all validation and any required local computation succeed. Errors remain visible in the editor with focus directed to the relevant section or item.

View application may perform local indexing or materialization through the application's Tokio runtime, but it never calls the source-query replacement path. Existing progress reporting and cancellation behavior apply to long-running local work.

### Preserve modal boundaries

Source Configuration remains the only runtime editor for adapter-dependent operations. Saved View remains the YAML serialization and persistence workflow. View Configuration may display read-only source-result context when useful, but it cannot mutate source state, and it does not become a persistence dialog.

## Risks / Trade-offs

- [A dense editor may be difficult to use in a small terminal] → Use section navigation, compact summaries, focused item editors, scrolling, and minimum-size fallback messaging.
- [Draft state can diverge if another command mutates the active view] → Treat the editor as modal, route keys exclusively to it, and snapshot active state again each time it opens.
- [Presentation fields currently live outside `ViewTransform`] → Use a draft that composes transform and presentation state, then commit them together through one application boundary.
- [Local Apply can be expensive for large file-backed sources] → Run required work on the standard Tokio runtime, expose progress/cancellation, and retain the prior active view until success.
- [Stable columns can disappear after reload] → Close or invalidate the draft on source-generation replacement and reject stale generation or column identities during Apply.

## Migration Plan

1. Introduce the draft model and validation without changing the existing modal entry point.
2. Replace the summary rendering and direct modal shortcuts with section and item interaction.
3. Commit validated drafts through the existing atomic view-application path.
4. Add synchronization, render, input, failure, and source-neutrality tests.
5. Update help and user documentation for the expanded `V` workflow.

The change is UI-local and requires no data migration. Reverting restores the summary modal while leaving source adapters and persisted saved views compatible.

## Open Questions

- Which presentation fields should be editable in the first iteration versus displayed read-only when the terminal is narrow?
- Should Apply remain a single explicit action for all sections, or should completed item editors update the modal draft while only the final Apply changes the table?
