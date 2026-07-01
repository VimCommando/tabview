## 1. Configuration Model

- [x] 1.1 Add a Cargo `saved-views` feature and optional saved-view dependencies, including `yaml_serde`, locale formatting support, SemVer parsing, and IP parsing support.
- [x] 1.2 Add saved view config data structures for view files, canonical file-stem names, filename patterns, column patterns, column visibility, column types, formats, widths, and alignment.
- [x] 1.3 Add YAML deserialization through an isolated config module using `yaml_serde`.
- [x] 1.4 Add semantic validation for filename regexes, filename globs, case-insensitive column wildcard patterns, type/format compatibility, POSIX-style top-level locale, and numeric masks.
- [x] 1.5 Add warning collection so invalid view files or properties are logged and reported to the TUI without aborting input loading.
- [x] 1.6 Add saved view model fields for up to three persisted ordered sort keys and filter state while keeping search state session-only.

## 2. Discovery and Matching

- [x] 2.1 Add saved view discovery for `tabview/views/*.yml` and `tabview/views/*.yaml` under `$XDG_CONFIG_HOME` or `~/.config` on every platform.
- [x] 2.2 Add test-only config root override support for deterministic saved view tests.
- [x] 2.3 Implement `.yml` before `.yaml` duplicate stem handling with logging and TUI warnings.
- [x] 2.4 Implement basename-only filename pattern classification for exact, glob, and regex patterns using platform filename case behavior.
- [x] 2.5 Implement deterministic view selection using exact-before-glob-before-regex precedence and lexicographic file path tie-breaking.
- [x] 2.6 Add CLI overrides for `--view <name>` forced selection and `--no-view` disabled selection, including extension normalization and conflict handling.

## 3. Column Resolution

- [x] 3.1 Resolve saved column configuration after header detection.
- [x] 3.2 Apply case-insensitive exact header matches before wildcard header matches.
- [x] 3.3 Implement wildcard tie-breaking by literal character count and lexicographic pattern order.
- [x] 3.4 Record warnings for configured columns that match no loaded header.

## 4. Table and View Integration

- [x] 4.1 Pass resolved saved view metadata into table/view initialization.
- [x] 4.2 Initialize column widths from numeric, `header`, `content`, `mode`, and `max` saved width values.
- [x] 4.3 Initialize data-cell alignment from saved alignment overrides or type-derived defaults.
- [x] 4.4 Initialize column visibility from saved `visible` values while preserving hidden columns in the table model.
- [x] 4.5 Preserve existing interactive width, gap, and sort controls after saved metadata is applied.
- [x] 4.6 Move existing all-column and current-column width commands from `c`/`C` to `z`/`Z`.
- [x] 4.7 Implement composable column hide/show commands under `c`, including count prefixes, directional suffixes, current-column hide, adjacent hidden-column show, and last-visible-column protection.
- [x] 4.8 Render `|` header indicators at visible boundaries where hidden source columns exist.

## 5. Formatting and Typed Behavior

- [x] 5.1 Implement display rendering for `plain`, string case transforms, system-locale numeric grouping, top-level locale overrides, and boolean display formats.
- [x] 5.2 Implement the first saved-view numeric mask grammar for `0`, `0.00`, `#,##0`, and `#,##0.00` style masks.
- [x] 5.3 Keep raw cell values separate from rendered display strings for yanking, reload, and internal table storage.
- [x] 5.4 Route type-aware sort behavior through saved column metadata where the subtype comparison is implemented, including ISO 8601 chronological sorting for `type: date`, loose SemVer parsing, and IPv4/IPv6 parsing for `type: ip`.
- [x] 5.5 Update `s/S`, `a/A`, and `#/@` sorting so repeated activations toggle the matching sort key, activated columns become primary, duplicate column entries are removed, and only the last three sort keys are kept.
- [x] 5.6 Add `csk`, `csj`, and `csx` column sort commands for ascending, descending, and clearing the current column from the sort list, with `cs` sort kind resolved to numeric for number-family columns and lexical for all other columns.
- [x] 5.7 Render `▲` and `▼` sort direction markers in visible sorted column headers.
- [x] 5.8 Update search and text/regex filters so saved-view formatted columns can match raw or rendered cell values.
- [x] 5.9 Update clipboard behavior so `y` yanks rendered current cell values and `Y` yanks raw current cell values.
- [x] 5.10 Implement POSIX locale formatting with grouping and decimal separators, system locale default, `en_US` fallback, and mask-over-locale behavior.

## 6. TUI Messages

- [x] 6.1 Add a VI-style footer notification/message line to the TUI.
- [x] 6.2 Route saved view warnings and errors to logs and the footer message line after loading.
- [x] 6.3 Add layout tests or snapshots proving the footer message line does not corrupt the table viewport.
- [x] 6.4 Add a `v` saved view modal that displays current view YAML plus loaded or placeholder target filename.
- [x] 6.5 Add modal actions for `s` save and `Esc` close.
- [x] 6.6 Add `y`/`n` overwrite confirmation when saving would replace an existing saved view file.
- [x] 6.7 Make the view modal read-only but scrollable when generated YAML exceeds the modal viewport.
- [x] 6.8 Disable the `v` binding and view saving when `--no-view` is active.

## 7. View Saving

- [x] 7.1 Serialize only non-default current runtime view state to schema-valid YAML, including modified widths, visibility, alignment, type, format, mask, explicit locale, ordered sort keys, and filters.
- [x] 7.2 Track whether the active view was loaded from a saved view file and retain its source path for save.
- [x] 7.3 Generate a placeholder save path by replacing only the opened input's last extension with `.yml` when no saved view was loaded.
- [x] 7.4 Create `config_dir/tabview/views` on save when the directory does not exist.
- [x] 7.5 Write saved view files atomically with a temporary file and rename or an equivalent failure-safe approach, and keep the modal open on save failures.
- [x] 7.6 Preserve the header comment block and matching inline comments when updating an existing saved view file.
- [x] 7.7 Save immediately and report a footer success message when the target file does not exist.
- [x] 7.8 Generate `filenames` with only the current input filename and omit top-level `locale` for auto-detected/default locale behavior.

## 8. Schema and Documentation

- [x] 8.1 Add the saved view schema to the implementation tree and include it in packaged sources.
- [x] 8.2 Reference the saved view schema from documentation.
- [x] 8.3 Add documentation for saved view feature gating, directory discovery, basename filename matching, `.yml` priority, CLI overrides, sparse YAML generation, ordered multi-level sort/filter persistence, view modal save flow, overwrite confirmation, column matching, column visibility, type aliases, formats, top-level locale, widths, `z`/`Z` width commands, composable column show/hide commands, hidden-column header indicators, raw/rendered yank, and warnings.
- [x] 8.4 Add an example saved view file based on a shard-style input.

## 9. Verification

- [x] 9.1 Add unit tests for YAML parsing, semantic validation, and warning behavior.
- [x] 9.2 Add unit tests for filename matching precedence, platform case behavior, `.yml` priority, and ambiguous match tie-breaking.
- [x] 9.3 Add unit tests for case-insensitive column matching precedence and wildcard tie-breaking.
- [x] 9.4 Add rendering tests for system locale formatting, top-level locale overrides, decimal separators, numeric masks overriding locale, string case transforms, width initialization, alignment, and saved column visibility.
- [x] 9.5 Add integration tests proving malformed saved views do not block opening valid input.
- [x] 9.6 Add tests for `--view`, `--no-view`, ISO 8601 date sorting, SemVer sorting, IPv4/IPv6 handling, boolean subtype parsing, raw/rendered yanking, `z`/`Z` width commands, composable column hide/show commands, hidden-column header indicators, shortcut sort toggling, last-three sort retention, `csk`/`csj`/`csx`, sort header markers, and raw-or-rendered search/filter matching.
- [x] 9.7 Add tests for the `v` modal loaded-file display, placeholder filename display, scroll behavior, disabled behavior under `--no-view`, immediate save, `y`/`n` overwrite confirmation, declined overwrite, directory creation, atomic writes, comment/order preservation, and save failure reporting.
- [x] 9.8 Run default-feature tests to verify the saved views feature gate does not break the default build.
- [x] 9.9 Run `cargo test --all-features`.
- [x] 9.10 Run `cargo clippy --all-targets --all-features -- -D warnings`.
