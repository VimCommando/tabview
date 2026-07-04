## Why

The Ratatui interface currently has hard-coded colors and modifiers, which makes the new table view harder to tune for different terminals and data domains. Users need a durable theme configuration surface that preserves the cmdzro visual baseline while allowing per-column conditional coloring for operational tables.

## What Changes

- Add user-configurable color themes loaded from YAML files under the tabview config directory.
- Ship a built-in default theme derived from `~/.config/nvim/colors/cmdzro.vim`, adapted for terminal table viewing: neutral gray cell text, cyan UI accents, blue reserved for UI elements, yellow reserved for search and UI emphasis, and red reserved for error or unhealthy states.
- Support theme colors in 16-color names, 256-color palette indexes, and 32-bit hex notation with graceful fallback for lower-color terminals.
- Add named theme tokens for table text, headers, selection, dividers, popups, footer messages, search highlights, warning/error states, and conditional cell styles.
- Extend saved view column definitions with conditional color formatting using `gradient`, `match`, `range`, and `identifiers` rules.
- Validate theme files and column color rules as non-fatal configuration input unless the user explicitly selects a missing or invalid theme by name.

## Capabilities

### New Capabilities
- `color-themes`: Theme file discovery, YAML schema, color parsing, color-mode fallback, built-in default theme, and application of theme tokens to Ratatui styles.

### Modified Capabilities
- `saved-views`: Column definitions can include conditional color formatting rules that apply theme colors, inline color values, or identifier palette colors to rendered cells.

## Impact

- Affected code: theme discovery/parsing module, Ratatui rendering style construction, saved view parsing/validation/schema, table view cell style lookup, and tests for terminal color fallback and conditional rules.
- Affected user API: new optional theme YAML files under the tabview config directory and new optional saved view column fields for conditional colors.
- Affected documentation: README/configuration examples and shipped schemas for theme YAML and saved view YAML.
- Dependencies: YAML parsing and color parsing support; reuse existing Ratatui `Color` types and existing config-directory conventions where practical.
