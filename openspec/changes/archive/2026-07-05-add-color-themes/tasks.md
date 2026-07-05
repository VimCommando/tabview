## 1. Theme Configuration

- [x] 1.1 Add theme/config dependencies and module structure for loading tabview YAML configuration and theme YAML files.
- [x] 1.2 Implement config root discovery for `$XDG_CONFIG_HOME/tabview` and `~/.config/tabview`.
- [x] 1.3 Implement built-in `cmdzro` theme data from the `cmdzro.vim` baseline.
- [x] 1.4 Implement theme discovery from `tabview/themes/*.yml` and `*.yaml` with deterministic theme names from file stems.
- [x] 1.5 Implement selected theme resolution from `config.yml`, with built-in default fallback and fatal errors for missing explicitly selected themes.

## 2. Color Parsing And Fallback

- [x] 2.1 Implement color parsing for 16-color names, 256-color palette indexes, aliases, and `#RRGGBBAA` hex values.
- [x] 2.2 Implement theme alias resolution with clear validation errors for unknown or cyclic aliases.
- [x] 2.3 Implement color-mode resolution for `auto`, `ansi16`, `ansi256`, and `hex32`.
- [x] 2.4 Implement deterministic fallback from truecolor to 256-color and 16-color outputs.
- [x] 2.5 Add unit tests for valid colors, invalid colors, alias resolution, and fallback mappings.

## 3. Theme Schemas And Validation

- [x] 3.1 Add a shipped theme schema or schema documentation for supported YAML fields and style tokens.
- [x] 3.2 Validate required theme style tokens and reject unsupported selected-theme tokens with useful paths.
- [x] 3.3 Treat malformed unselected theme files as logged non-fatal warnings.
- [x] 3.4 Document the default `cmdzro` theme constraints: no blue text, yellow only for search/UI emphasis, and red only for error/unhealthy states.

## 4. Ratatui Rendering

- [x] 4.1 Add a resolved theme object to the TUI startup path and pass it into UI rendering functions.
- [x] 4.2 Replace hard-coded table location, cell, divider, header, selection, and hidden-marker styles with theme token lookups.
- [x] 4.3 Replace hard-coded footer and popup styles with theme token lookups.
- [x] 4.4 Apply themed search highlight styling without changing search matching behavior.
- [x] 4.5 Add UI buffer tests proving themed styles are used for table chrome, popups, footer messages, and search highlights.

## 5. Conditional Column Colors

- [x] 5.1 Extend saved view data structures and YAML parsing with ordered column `colors` rules.
- [x] 5.2 Extend `schemas/view.schema.json` for `gradient`, `match`, and `range` conditional color rules.
- [x] 5.3 Implement first-match-wins conditional style evaluation per rendered cell.
- [x] 5.4 Implement fixed gradients with inclusive start and exclusive next-stop ranges.
- [x] 5.5 Implement auto gradients over parseable numeric column min/max values with `steps` defaulting to `8`.
- [x] 5.6 Implement discrete `match` rules for boolean, numeric, and string-family values.
- [x] 5.7 Implement numerical `range` rules with `lt`, `lte`, `gt`, and `gte` bounds and uncolored gaps.
- [x] 5.8 Implement string-mode `identifiers` rules using stable unique-value color assignment from theme-level or view-level color families.
- [x] 5.9 Preserve raw/rendered values for sorting, filtering, searching, copying, and popups when conditional colors apply.
- [x] 5.10 Add tests for invalid conditional rules being non-fatal saved view warnings.

## 6. Serialization And Documentation

- [x] 6.1 Decide whether saved view modal serialization preserves loaded conditional color rules and document the behavior.
- [x] 6.2 Add README examples for `config.yml`, theme YAML files, and saved view conditional color rules.
- [x] 6.3 Add sample theme and saved view fixtures covering ansi16, ansi256, hex32, gradient, match, and range examples.

## 7. Verification

- [x] 7.1 Run `cargo test` with default features.
- [x] 7.2 Run `cargo test --features saved-views`.
- [x] 7.3 Run schema validation or fixture tests for theme YAML and saved view YAML examples.
- [x] 7.4 Manually smoke-test the TUI with the built-in default theme and at least one user theme.
