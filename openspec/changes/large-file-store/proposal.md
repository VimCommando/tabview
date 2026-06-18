## Summary

Add true large-file support by moving the interactive viewer from an eagerly materialized `Vec<Vec<String>>` table to a store-backed table model that can open large files incrementally, index rows on demand, and materialize only when an operation requires it.

## Motivation

The Rust rewrite includes a 100 MiB lazy threshold, a `TableStore` trait, and a prototype `LazyFileTable`, but the live TUI still reads and parses the full input before rendering. That is acceptable groundwork for the first replacement, but it does not satisfy the expected user experience for very large files.

Large-file support should be a separate change because it affects the core view model, row access, column profiling, search, sort, reload, status reporting, and tests. Treating it as follow-on work keeps the rewrite archivable while giving this behavior focused acceptance criteria.

## Goals

- Open files at or above the 100 MiB threshold without parsing the entire file before the first render.
- Keep ordinary small-file behavior simple and in-memory.
- Preserve existing CLI, keybindings, parsing behavior, TUI layout, and current table operations.
- Rework `TableView` around a store-backed row access boundary suitable for future features.
- Build row offset indexes incrementally as navigation, search, and viewport rendering require more rows.
- Make full-table operations such as sort use explicit controlled materialization or indexing.
- Provide user-visible non-fatal status for long-running materialization/indexing work.

## Non-Goals

- No new editing, filtering, projection, formula, or data transformation features.
- No asynchronous runtime requirement unless implementation proves it is needed.
- No mmap-only design that excludes stdin, compressed streams, or non-seekable future inputs.
- No change to the `tabview` CLI surface except documentation of large-file behavior.

## User Impact

Users opening large files should see the first screen quickly rather than waiting for complete parsing. Navigation through already indexed rows should remain responsive. Operations that need the whole table may take time, but they should be explicit, controlled, and should not corrupt viewer state.
