## Context

`tabview` currently loads a single tabular input and lets users adjust view state interactively. There is no persistent configuration layer for file-specific column metadata, so repeated operational views must be recreated every session.

Saved views add a small user configuration surface under `~/.config/tabview/views/`. A view is selected by matching the opened input filename, then its sparse column configuration is applied to headers present in the loaded table.

The existing codebase already depends on `regex` and has table, view, sort, and UI modules. This change should add a focused config module behind a Cargo `saved-views` feature and pass resolved metadata into existing table/view initialization rather than spreading config file parsing through the TUI.

## Goals / Non-Goals

**Goals:**
- Load `*.yml` and `*.yaml` saved view files from the user's tabview config directory.
- Treat each saved view file as one view whose unique name is the filename stem.
- Validate saved views with strongly typed Rust structures plus semantic validation for regexes, globs, masks, and enum combinations.
- Ship a schema file that YAML-aware editors and tests can use to validate view files and include it in documentation.
- Apply sparse column metadata case-insensitively by exact header first, then wildcard header patterns.
- Use saved type metadata for display formatting, alignment defaults, and type-aware sorting where applicable.
- Display saved view warnings and errors through logs and a VI-style footer notification/message line in the TUI.
- Let users inspect and save the current view configuration from a `v` modal.
- Preserve current interactive controls; saved values seed initial state and do not lock the user out of changing widths or sorting.

**Non-Goals:**
- Free-form YAML editing inside the TUI.
- Multiple active views or layered view inheritance.
- Path-based project profiles beyond filename matching.
- Full Excel-compatible custom number formatting.

## Decisions

### Configuration File Format

Use YAML files with this shape:

```yaml
name: cat-shards
locale: en_US
filenames:
  - cat_shards.txt
  - "*shards*"
  - "^cat_.*txt$"
columns:
  index:
    type: string
    format: plain
    width: 20
    visible: true
  shard:
    type: number
    format: plain
    width: header
    align: left
  "*count":
    type: number
    format: locale
    width: content
  segment:
    type: text
sort:
  - column: shard
    direction: asc
    kind: numeric
  - column: index
    direction: asc
    kind: natural
filters:
  - column: "*count"
    action: in
    kind: numeric
    condition: ">0"
```

The user-facing `type` accepts both broad types and subtype aliases:
- String family: `string`, `text`, `date`
- Number family: `number`, `float`, `int`, `semver`
- IP family alias: `ip`, internally treated as a string-family subtype with IPv4 and IPv6 parsing/comparison support
- Boolean family: `boolean`, `char`, `bit`, `word`

Internally, deserialize these aliases to `ColumnType::{String(StringKind), Number(NumberKind), Boolean(BooleanKind)}`. Broad aliases map to defaults: `string -> text`, `number -> float`, `boolean -> word`. `ip` maps to `StringKind::Ip`, not a number subtype, because IPv6 addresses are not meaningfully numeric for this viewer.

Alternative considered: separate `type: string` and `subtype: date`. That is more explicit but noisier, and it conflicts with the rough `type: text` shorthand. The typed enum can still support a future `subtype` field if needed.

The canonical view name is the filename stem, not a separate field. `cat-shards.yml` defines view `cat-shards`; `--view cat-shards`, `--view cat-shards.yml`, and `--view cat-shards.yaml` all normalize to the same view name. The YAML `name` field is required so the file contents remain complete even when copied away from the filename. It should match the filename stem in examples, but runtime selection is based on the stem to guarantee uniqueness.

Each file contains exactly one view definition. If both `cat-shards.yml` and `cat-shards.yaml` exist, load `cat-shards.yml`, ignore `cat-shards.yaml`, log the conflict, and display a warning through the TUI message line.

### YAML Parser

Use `yaml_serde` for typed YAML deserialization. Its docs describe it as an actively maintained fork of `serde-yaml` published by the YAML organization, and it exposes the Serde APIs this feature needs, including `from_str`, `from_reader`, `from_slice`, and derive-based struct deserialization.

`serde_yaml` and `serde_yml` are both currently marked deprecated/unmaintained in their docs, so do not make either the first choice. `serde-saphyr` remains a reasonable fallback if `yaml_serde` proves unsuitable during implementation. `yaml-rust2` is viable for raw YAML parsing but would require more manual mapping into typed config.

Alternative considered: TOML. It would reduce YAML parser dependency risk, but the requested file format is YAML and YAML is friendlier for sparse nested column maps.

### Discovery and Matching

Discover files in `config_dir/tabview/views`, where `config_dir` is the platform config directory. On Linux this resolves to `~/.config`; in tests, allow overriding the config root so behavior is deterministic. Discovery is compiled and run only when the `saved-views` Cargo feature is enabled.

Each entry in `filenames` is classified as:
- Regex if it starts with `^` or ends with `$`.
- Glob if it contains glob metacharacters `*`, `?`, or `[`.
- Exact otherwise.

Matching applies only to the opened input basename, not to parent directories or full paths. Exact and glob matching follow platform filename case behavior: case-insensitive on case-insensitive platforms, case-sensitive on case-sensitive platforms. Regex matching uses the same platform-derived case behavior by compiling with case-insensitive mode where appropriate.

Exact matches outrank glob matches, and glob matches outrank regex matches. If multiple views match with the same rank, choose the lexicographically first view file path and emit a non-fatal warning.

Alternative considered: require explicit tagged objects such as `{ glob: "*shards*" }`. That is less ambiguous, and it may be worth adding later, but the string-only form keeps the initial format compact.

### CLI Overrides

Saved views apply automatically by default when the `saved-views` feature is enabled. Add `--view <name>` to force a saved view by canonical filename-stem view name and `--no-view` to disable saved view loading for the current invocation.

`--view <name>` normalizes away a trailing `.yml` or `.yaml`, so users may pass either `--view cat-shards` or `--view cat-shards.yml`. It should still load and validate view files, but selection is by exact canonical view name rather than filename match. If no saved view has that name, startup should fail with a clear CLI error because the user explicitly requested the view. `--no-view` should skip discovery entirely. If both flags are provided, argument parsing should reject the invocation.

### Column Matching

Column keys under `columns` are matched case-insensitively against header names after the header row is known.

Precedence:
1. Exact header key.
2. Wildcard header key using glob-style matching.
3. No saved metadata.

Exact matches take priority even if a wildcard also matches. When multiple wildcard keys match one header, choose the most specific pattern by literal character count, then lexicographic key order, and emit a warning if the tie is exact.

### Width and Alignment

`width` accepts either a positive integer character count or one of:
- `header`: initialize to the display width of the column header.
- `content`: initialize to the max display width of visible/materialized content.
- `mode`: use existing mode-based sizing for that column.
- `max`: use existing max-content sizing for that column.

`align` accepts `left` or `right`. If omitted, number-family columns default to right alignment and other columns default to left alignment. Headers remain left-aligned unless a later design explicitly adds header alignment.

### Column Visibility and Keybindings

Each column config may set `visible: true|false`. Omitted `visible` defaults to `true`. Hidden columns remain in the table model and raw data, but are excluded from viewport rendering, horizontal navigation, visible-column width calculations, search traversal, and yanking unless a command explicitly targets hidden columns. Filters may still apply to hidden columns when a filter is already active for that source column, but the first implementation does not need a UI for creating a new filter on a hidden column.

Free `c` for composable column commands by moving the existing column width bindings:
- `[num]z`: existing `[num]c` behavior, toggling all-column width mode or setting all columns to a fixed width.
- `[num]Z`: existing `[num]C` behavior, maximizing the current column or setting the current column to a fixed width.

Use `c` as the column command prefix:
- `[num]chh`: hide visible columns to the left of the current column.
- `[num]chl`: hide visible columns to the right of the current column.
- `[num]chj` and `[num]chk`: hide the current column.
- `[num]cHh`: show hidden columns immediately adjacent to the left of the current visible column in source order.
- `[num]cHl`: show hidden columns immediately adjacent to the right of the current visible column in source order.
- `[num]cHj` and `[num]cHk`: show the current source column only when the cursor can still refer to a hidden current source column after reload or saved-view application; otherwise they are no-ops with a footer message.

The numeric prefix counts columns. For horizontal hide/show commands, the count applies to adjacent columns on the requested side, nearest first. Show commands only reveal contiguous hidden columns adjacent to the current visible column on the requested side; they do not skip across visible columns. For current-column commands, the count is ignored because there is only one current column target. The viewer must prevent hiding the last visible column and report a footer message instead.

Render a `|` separator between visible headers when one or more hidden source columns exist between those visible headers. Also render the indicator at the viewport edge when hidden columns exist beyond the first or last visible header. This gives users a visible hint that `cHh` or `cHl` can reveal adjacent hidden columns.

### Sort and Filter State

Saved views persist sort and filter state, but not search state. Sort state is an ordered list to support multi-level sorting. Each sort entry should capture the source column, direction (`asc` or `desc`), and sort kind. The list order is the precedence order, with earlier entries acting as higher-priority sort keys. Filter state should capture the source column, action (`in` or `out`), filter kind (`text`, `regex`, or `numeric`), and condition string.

Interactive sort shortcuts maintain this ordered list with at most three entries. Activating `s/S`, `a/A`, or `#/@` on a column makes that column the primary sort key, keeps the previous sort keys as trailing lower-priority keys, removes duplicate entries for the same source column, and drops entries beyond the last three. Repeating the same shortcut on a column that is already sorted with the same kind and direction toggles that sort key off.

Use `c` composable sort commands for explicit column sort management:
- `csk`: sort current column ascending.
- `csj`: sort current column descending.
- `csx`: clear the current column from the sort list.

`csk` and `csj` choose the sort kind from the resolved column data type: number-family columns use numeric sorting, and all other columns use lexical sorting. Clearing with `csx` is the explicit way to remove a column sort without relying on shortcut toggle behavior.

Headers for sorted columns render ASCII direction markers: `^` for ascending and `v` for descending. The marker appears on every sorted visible column so users can see multi-level sort participation.

When serializing generated YAML, include sort and filter state only when the user has an active sort or active filters. Search remains session-only and must not be saved.

### View Modal and Saving

Bind `v` to a saved view modal when the `saved-views` feature is enabled. The modal displays:
- The current view configuration as YAML.
- The source filename when the view was loaded from a saved view file.
- The target filename when the view was not loaded from a file, using the opened input filename with only the last extension replaced by `.yml` under `config_dir/tabview/views/`.
- Available actions: `s` to save and `Esc` to close.

The modal is read-only YAML, but scrollable when the generated content is larger than the modal viewport.

The displayed YAML should be generated from the current runtime view state, not only the initially loaded file. It should be sparse: include `name`, `filenames`, and only values that differ from defaults or represent explicit view state. Include top-level `locale` only when the user configured an explicit locale rather than auto-detected/default locale. Include a column only if its view state was modified, including width, visibility, alignment, type, format, mask, sort participation, or active filter participation. Interactive changes such as hiding/showing columns and width adjustments should be reflected in the generated YAML. Generated YAML should include only the current input filename in `filenames`, not the originally loaded view's filename patterns.

Saving writes to the loaded view file when the view came from disk. When updating an existing file, preserve comments and field order so the saved file can remain a self-documenting example. If the current view did not come from disk, saving writes to `~/.config/tabview/views/<input-name-with-last-extension-replaced>.yml`. For example, `cat_shards.txt` becomes `cat_shards.yml` and `foo.bar.csv` becomes `foo.bar.yml`. The implementation should create the view directory if it does not exist.

If the target file does not exist, pressing `s` saves immediately and reports success in the footer notification line. If the target file already exists, the modal must ask for overwrite confirmation with `y` and `n` before writing. A declined overwrite returns to the modal without changing the file. A successful save updates the active view source path and reports success through the footer message line.

Saves must be atomic, using a temporary file in the destination directory followed by rename or an equivalent failure-safe approach. Save failure keeps the modal open, logs the error, and shows a modal/footer warning.

If the user invoked `--no-view`, saved view loading and authoring are disabled for that session. The `v` binding is unavailable and view saving must not run.

### Formatting and Number Masks

`format` controls display only. It must not mutate raw cell values used for reload, yanking, or persisted data. Sorting may use parsed typed values, but display formatting should remain a separate rendering step. Search and text/regex filtering should consider both raw and rendered values so a user can find either `1000` or `1,000` when locale formatting is active.

Initial formats:
- All types: `plain`
- String family: `uppercase`, `lowercase`
- Number family: `locale`, `mask`
- Boolean family: `char`, `bit`, `word`

For user-definable numeric masks, use a small tabview-owned mask grammar for the first implementation:
- `0`
- `0.0`, `0.00`, etc. for fixed decimal places.
- `#,##0`
- `#,##0.0`, `#,##0.00`, etc. for grouped fixed decimals.

Represent this as:

```yaml
columns:
  latency:
    type: float
    format: mask
    mask: "0.00"
```

This maps cleanly to Rust's dynamic precision formatting for decimal places and `num-format` for integer grouping. It is easier to validate and document than an Excel-compatible formatter, and it avoids precision surprises from broad `f64`-only formatting DSLs for large integers.

`format: locale` uses the POSIX-style system locale by default and falls back to `en_US` if system locale detection or lookup fails. A saved view may set top-level `locale: <identifier>` with POSIX-style names such as `en_US` to override system locale for that view. Keep locale at the view level for the first implementation so every locale-formatted column in a view uses the same grouping and decimal separator behavior.

Locale formatting applies grouping and decimal separators. Numeric masks override locale formatting; a mask such as `#,##0.00` renders according to the mask grammar rather than swapping separators based on locale.

`type: date` parses ISO 8601 date/time strings in the first implementation. Date sorting should be chronological for parseable ISO 8601 values and should fall back consistently for non-parseable values.

`type: semver` accepts loose versions that the chosen SemVer parser can parse, including common forms beyond strict `MAJOR.MINOR.PATCH` when the parser supports them. `type: ip` accepts IPv4 and IPv6 and compares parseable addresses using IP-aware ordering while treating them as string-family display values.

Boolean parsing accepts common examples by subtype:
- `word`: `true`/`false`, `yes`/`no`
- `bit`: `1`/`0`
- `char`: `y`/`n`

Yanking uses rendered values by default: `y` copies the rendered current cell and `Y` copies the raw current cell.

Alternatives considered:
- Rust standard formatting strings such as `.2` or `.2f`: very Rust-friendly and easy to implement, but less friendly for users asking for `0.00`.
- `format_num`: supports Python-style specs and comma grouping, but its docs note values must convert into `f64`, which can lose precision for very large numbers.
- `rust_decimal`: useful if decimal correctness becomes important for financial-style data, but it is heavier than needed for display-only rounding in this viewer.

### Schema File

Ship a JSON Schema file named `schemas/view.schema.json` in the implementation. YAML language servers can apply JSON Schema to YAML files, and the same file can validate examples in tests.

The schema should validate structural shape and enums, and should be installed/distributed with the crate and referenced from documentation. Runtime validation still needs Rust code for rules JSON Schema cannot fully express cleanly, such as regex compilation, glob compilation, filename conflict precedence, filename stem/name consistency, and mask parsing.

### Error Handling

Saved view errors are non-fatal unless the user explicitly requested a missing view through `--view`. A bad config file must not prevent opening data. The loader should collect warnings, log them, and make them visible inside the TUI through a VI-style footer notification/message line without corrupting table state.

Examples:
- Invalid YAML: ignore that view file and warn.
- Invalid regex or mask: ignore that pattern or column property and warn.
- Unknown type or format: ignore the affected column property and warn.
- Duplicate `.yml`/`.yaml` view stems: load `.yml`, ignore `.yaml`, log and warn.
- Save failure: keep the modal open, log the error, and show a footer/modal warning.
- No matching view: open normally without warning.

## Risks / Trade-offs

- Ambiguous string patterns can surprise users -> Document the exact/glob/regex classification and add tests for edge cases.
- YAML parser dependencies are in flux -> Prefer a maintained Serde-compatible parser and isolate it behind a config module.
- Locale-aware formatting can add platform complexity -> Use system locale by default, allow top-level `locale`, and isolate locale lookup/format selection behind formatting helpers.
- Display formatting can diverge from sorting values -> Keep raw values, parsed typed values, and rendered strings as separate concepts in the table/view model.
- Feature-gated code can drift from default builds -> Include `--features saved-views` in verification and keep default build tests passing.
- Regex and glob matching at startup adds work -> Compile view matchers once during config loading; the expected number of view files is small.

## Migration Plan

1. Add the `saved-views` Cargo feature, optional dependencies, config data structures, loader, matcher, and schema.
2. Integrate resolved view metadata into table/view initialization.
3. Add CLI override flags for forced view selection and disabling saved views.
4. Add display formatting, typed sort hooks, raw/rendered search-filter matching, rendered/raw yank behavior, and column visibility behavior behind saved metadata.
5. Move width keybindings from `c`/`C` to `z`/`Z` and add composable column show/hide commands under `c`.
6. Add the `v` view modal, current-view YAML generation, save action, and overwrite confirmation.
7. Add tests using a temporary config directory.
8. Document the saved view directory, examples, and schema path.

Rollback is simple: if config loading is disabled or fails, `tabview` keeps current behavior.

## Open Questions

- Should ambiguous multiple matching views only warn and select deterministically, or should a later release add an interactive selector?
- Should wildcard column keys support only globs, or should regex column keys be added too?
