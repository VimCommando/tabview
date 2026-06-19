## Why

Users often reopen recurring operational files and reapply the same column widths, typing assumptions, and display preferences by hand. Saved views let users define reusable YAML configuration files that match input filenames and apply known view state automatically.

## What Changes

- Add user-defined saved view files under `~/.config/tabview/views/*.yml` and `*.yaml`.
- Match saved views automatically to opened file basenames by exact filename, glob, or regular expression patterns.
- Gate saved view support behind a Cargo `saved-views` feature.
- Add CLI overrides to force a named saved view or disable saved view loading for a run when saved views are enabled.
- Treat the view file stem as the unique view name, with one view definition per file.
- Add a `v` keybinding that opens a view modal showing the current view configuration and source/destination filename.
- Allow saving sparse current view configuration from the view modal into `~/.config/tabview/views/`, with immediate save for new files, `y`/`n` overwrite confirmation for existing files, and atomic writes.
- Allow sparse per-column configuration keyed by exact header or wildcard header patterns, matched case-insensitively.
- Allow saved views to set per-column visibility with `visible: true|false`.
- Persist sort and filter state in saved views while keeping search session-only.
- Add column type metadata for string, number, and boolean values with explicit subtypes for text, date, float, int, semantic version, IP address, char, bit, and word booleans; IP is a string-family subtype with IPv4 and IPv6 support.
- Add display formatting metadata, including plain formatting, POSIX system-locale numeric grouping and decimal separators with a top-level `locale` override, string case transforms, and a Rust-friendly numeric mask option.
- Add width and alignment metadata that can seed the existing column layout while preserving interactive adjustments.
- Provide a YAML schema file so view configurations can be validated by editors, tests, and future CLI tooling.
- Report invalid saved view files non-fatally through logs and a VI-style footer notification/message line in the TUI.

## Capabilities

### New Capabilities
- `saved-views`: Loading, validating, matching, and applying user-defined saved view YAML files.

### Modified Capabilities

- `cli-compatibility`: Add saved view override flags.
- `filters`: Allow text and regex filters to match raw or rendered cell values when saved view formatting is active, and persist active filters in saved views.
- `table-operations`: Allow search to match raw or rendered cell values when saved view formatting is active, distinguish rendered versus raw yank, add column hide/show operations, and move existing `c`/`C` width commands to `z`/`Z`.
- `tui-navigation`: Add a footer notification/message line for saved view warnings and errors, plus a saved view modal bound to `v`.

## Impact

- Adds configuration discovery under the user's config directory, likely using the platform config directory plus `tabview/views`.
- Adds optional `yaml_serde`, locale formatting, SemVer parsing, and IP parsing support behind the saved views feature.
- Extends table initialization with optional view-derived column metadata for width, alignment, formatting, type-aware sorting, and display rendering.
- Adds serialization of the current runtime view state back to saved view YAML.
- Adds a JSON Schema or YAML-compatible schema artifact distributed with the project and included in documentation.
- Adds tests for config discovery, filename matching precedence, schema validation, column matching precedence, CLI overrides, formatted search/filter behavior, and non-fatal error handling.
