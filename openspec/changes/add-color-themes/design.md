## Context

`tabview` renders the Ratatui interface from a small set of hard-coded `Style::default()` values in `src/ui/mod.rs`. Saved views already provide sparse column metadata under `~/.config/tabview/views`, and the current schema covers column type, formatting, width, alignment, visibility, sort, and filter state.

Color themes add a second configuration surface under the same tabview config root. The default visual language should be derived from `cmdzro.vim`: dark background, neutral gray cell text, cyan UI accents, blue only for UI surfaces, yellow only for search or UI emphasis, and red only for errors or unhealthy states.

## Goals / Non-Goals

**Goals:**
- Load named YAML themes from `$XDG_CONFIG_HOME/tabview/themes` or `~/.config/tabview/themes`.
- Provide a built-in `cmdzro` default theme so the TUI is themed even when no config exists.
- Parse color values from 16-color names, 256-color indexes, and `#RRGGBBAA` hex values.
- Resolve colors to terminal-capable Ratatui colors with predictable fallback.
- Replace hard-coded UI styles with semantic theme tokens.
- Extend saved view columns with conditional color rules for numerical gradients, discrete matches, and numerical ranges.

**Non-Goals:**
- No live theme reloading during an active TUI session.
- No alpha blending in terminal rendering; hex alpha is accepted and ignored for cell output.
- No conversion of Vim colorscheme files at runtime. `cmdzro.vim` is the design baseline for the built-in theme, not a loaded dependency.
- No semantic inference of unhealthy states beyond explicit theme tokens and user-defined conditional rules.

## Decisions

### Theme file shape

Use YAML for themes because style tokens are naturally hierarchical and read better as nested maps than as many repeated table sections. Theme files are YAML-only; there is no legacy TOML theme compatibility path.

Example:

```yaml
name: cmdzro
mode: auto # auto | ansi16 | ansi256 | hex32

palette:
  text: "#AFAFAFFF"
  muted: palette(242)
  ui_blue: palette(19)
  cyan: dark-cyan
  green: dark-green
  magenta: magenta
  yellow: yellow
  error: dark-red

identifiers:
  colors: [bright-green, magenta, cyan, white]

styles:
  table:
    cell:
      fg: text
    header:
      fg: text
      modifiers: [bold]
    selected:
      fg: text
      bg: ui_blue
  search:
    highlight:
      fg: yellow
      modifiers: [underline]
  message:
    error:
      fg: bright-white
      bg: error
```

Theme tokens should be semantic rather than copying Vim highlight group names. The renderer asks for `table.header`, `popup.border`, `search.highlight`, or `message.error`, not `PMenuSel` or `StatusLine`.

Alternative considered: load Vim colorschemes directly. Rejected because Vim highlight groups do not map cleanly to table-specific conditional styles, and runtime Vimscript parsing would add avoidable complexity.

### Color representation

Introduce an internal `ThemeColor` enum for configured values:

- `Ansi16(ColorName)`
- `Ansi256(u8)`
- `Rgb { r: u8, g: u8, b: u8, a: u8 }`
- `Alias(String)`

Theme resolution validates aliases once at load time and converts resolved colors to `ratatui::style::Color`. `#RRGGBBAA` preserves alpha in the parsed representation for diagnostics and future compatibility, but terminal rendering uses only RGB.

Fallback should be deterministic:

- `hex32` on truecolor terminals uses `Color::Rgb`.
- `hex32` on 256-color terminals maps to the nearest xterm-256 color.
- `hex32` or `ansi256` on 16-color terminals maps to the nearest configured 16-color fallback.
- 16-color values use tabview's built-in cmdzro base palette; in truecolor and 256-color modes they resolve through those RGB values, while explicit `ansi16` mode emits ANSI colors for the terminal palette.

Alternative considered: store only Ratatui `Color` immediately. Rejected because doing so loses the original mode, alpha, and alias information needed for validation messages and terminal fallback.

### Configuration selection

Add a small top-level YAML config file for tabview runtime settings, likely `$XDG_CONFIG_HOME/tabview/config.yml` or `~/.config/tabview/config.yml`:

```yaml
theme: cmdzro
```

The first implementation can load only `theme`; future settings can extend the same file. If the config file is missing, use the built-in `cmdzro` theme. If the config file selects a theme that cannot be loaded, fail before entering raw terminal mode with a clear error.

Alternative considered: add a CLI flag only. Rejected as the primary surface because the user requested file-based configurability. A CLI override can be added later without changing the theme model.

### Renderer integration

Create a `Theme` or `ResolvedTheme` object before initializing the view loop and pass it into UI rendering functions. Replace direct hard-coded styles in `src/ui/mod.rs` with token lookups:

- table location, current cell text, divider, header, ordinary cell, selected cell, hidden marker
- footer/info/warning/error messages
- popup background, border, title, body, disabled text, active item, actions
- search prompt and search highlight
- filter/column-info modal states

Keep all UI rendering functions pure over `Buffer` as they are today; the theme object should provide styles, not own rendering.

### Conditional color rule model

Saved view columns get an ordered `colors` list. Each item has one rule kind and one style result:

```yaml
columns:
  disk.used_percent:
    type: number
    colors:
      - range:
          "<10": red
          ">=90": red
      - gradient:
          mode: fixed
          stops:
            10: green
            50: yellow
            75: magenta
  active:
    type: boolean
    colors:
      - match:
          true: green
          false: muted
  ip:
    type: ip
    colors:
      - identifiers:
          colors: auto
  host:
    type: string
    colors:
      - identifiers:
          colors: [cyan, "palette(198)", "#25A39AFF"]
```

Use first-match-wins within a column. A matched conditional color overrides the cell foreground but does not replace selected-cell background or other readability-critical selection styling.

Alternative considered: put conditional color rules in theme files. Rejected because the rules are data/domain-specific and belong beside saved view column definitions.

### Gradient semantics

`mode: fixed` requires explicit numeric stop values in a `stops` map where each key is the stop value and each value is the color. Each stop owns the half-open interval from its value inclusive to the next stop exclusive; the last stop includes all values greater than or equal to its value. Values below the first stop are uncolored.

`mode: auto` calculates buckets from observed parseable numeric values in the column. It accepts two or more colors and optional `steps`, defaulting to `8`. Non-numeric values do not affect min/max and remain uncolored unless another rule matches. Auto gradients can interpolate between RGB values in truecolor mode and otherwise choose nearest configured palette colors per bucket.

`identifiers` is a string-mode rule. It maps each unique rendered value in the source column to a stable identifier index, then resolves that index through color families. `colors: auto` uses the active theme's `identifiers.colors`; a saved view can override the families with an explicit color array. Each family generates 16 dark-to-light shades, and identifiers cycle across families before advancing shades. The darkest generated shade is clamped to the ANSI dark/dim foreground equivalent for contrast on black backgrounds.

### Schema and validation

Add a theme schema under `schemas/theme.schema.json` or equivalent documentation for YAML-aware editors. Extend `schemas/view.schema.json` with `colors` definitions for `gradient`, `match`, and `range`.

Validation should stay consistent with saved views:

- malformed selected theme: fatal before TUI starts
- malformed unselected theme: warning only
- malformed conditional color rule: warning, ignore that rule
- unknown color alias in an applied rule: warning, ignore that rule

## Risks / Trade-offs

- Color fallback may surprise users on limited terminals -> expose the requested mode and resolved mode in diagnostics or info output.
- Conditional styling can reduce selection readability -> selection background/modifier remains authoritative and tests cover selected conditionally-colored cells.
- Auto gradients require scanning column values -> compute color buckets after initial column type/format metadata is applied and cache per-column rule state.
- YAML parsing is now part of the core theme path -> keep the schema narrow and avoid feature-gating the core TUI theme path.
- Saved view schema grows more complex -> keep conditional rules as an ordered list with one rule kind per item and targeted validation errors.
