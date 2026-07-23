# Tabview

View delimited text, JSON, and NDJSON files in a spreadsheet-like terminal
interface.

**This project is functional but future development will be sporadic and
limited. For a more fully featured CSV viewer/spreadsheet app, check out the
[Visidata project](https://github.com/saulpw/visidata).**

Posted by Scott Hansen <tech@firecat53.net>

Original code forked from <http://www.amk.ca/files/simple/tabview.txt>.

Contributed by A.M. Kuchling <amk@amk.ca>.

Other contributors:

- Matus Gura <matus.gura@gmail.com>
- Nathan Typanski <ntypanski@gmail.com>
- Sébastien Celles <s.celles@gmail.com>
- Yuri D'Elia <wavexx@thregr.org>

The highlighted position is shown in the top-left corner of the screen. The
contents of that cell are shown next to it.

## Features

- Rust command-line application distributed as the `tabview` binary.
- Spreadsheet-like view for visualizing tabular data.
- Automatic or explicit delimited, JSON, and NDJSON input selection.
- RFC 6901 JSON Pointer selection for tables embedded in response documents.
- Incremental indexing for large seekable inputs and typed JSON scalar values.
- Vim-like navigation, including `h`, `j`, `k`, `l`, `g`, `G`, marks, and
  numeric prefixes such as `12G`.
- Persistent header row toggling.
- Lexical, natural, and numeric sorting by the current column.
- Dynamic column width and gap adjustment.
- Full-text incremental search with `n` and `N` result navigation.
- Current-column filter-in and filter-out with text, regex, and numeric modes.
- Full-cell popup with `Enter`.
- Optional clipboard support for yanking the current cell.
- In-place reload when data changes.
- Built-in keybinding help with `F1` or `?`.
- Example screenshots in the `screenshots/` directory.

## Requirements

- Rust toolchain for installation with Cargo.
- Optional clipboard support can be enabled with the `clipboard` Cargo feature.
- Saved views are enabled by default. Build with `--no-default-features` to omit
  saved view support.

## Installation

Install the latest published release:

```sh
cargo install tabview
```

Install from a local checkout:

```sh
cargo install --path .
```

Build with clipboard support:

```sh
cargo install tabview --features clipboard
```

## Usage

From the command line:

```sh
tabview <filename>
tabview <filename> --start_pos 6,5
tabview <filename> +6:5
tabview <filename> --encoding iso8859-1 +6:
tabview <filename> --delimiter '\t' --quoting QUOTE_NONE
tabview <filename> --width mode
tabview <filename> --width max
tabview <filename> --width 20
tabview <filename> --view cat-shards
tabview <filename> --no-view
tabview response.json --json-path /hits/hits
tabview repositories.json --object-mode entries
tabview settings.json --object-mode record
tabview records.ndjson --format ndjson
tabview response.data --format json --schema-scan full
tabview data.csv --output table
tabview --interactive data.csv
tabview --interactive --output table data.csv > edited.txt
```

Read from standard input:

```sh
cat data.csv | tabview -
cat records.ndjson | tabview --format ndjson -
cat data.csv | tabview --output table - > table.txt
cat data.csv | tabview --interactive --output table - > edited.txt
```

Runtime and serialization are separate. `--interactive`/`-i` forces the TUI;
`--output table`/`-o table` selects fixed-width text output. Combining them runs
the TUI and writes the final live view after a normal quit. Hiding columns,
formatting, filtering, and sorting in that session affect the emitted result.
Using `-i` without `-o` is view-only and writes no final table.

With neither option, terminal stdout selects the TUI and redirected or piped
stdout selects plain table output automatically. Table output never uses raw
mode or the alternate screen. It emits every configured row and visible column,
uses no aggregate terminal-width limit, and leaves wrapping, paging, or
truncation to downstream tools. `--color auto` and `--color never` produce plain
bytes; `--color always` opts into theme-derived ANSI styling.

When stdin supplies table data during interactive operation, Tabview uses the
controlling terminal for UI input and drawing. It continues draining finite
stdin in the background while the TUI is active, and an explicit final export
waits for EOF so late rows and columns are included. Redirect output to a
different path from the input: shells truncate redirection targets before Tabview starts.
The current `table` format is fixed-width text and does not preserve CSV or JSON
syntax, so write it to a text destination rather than replacing the source.
Future serializers such as CSV and Markdown can be added as new `--output`
values without changing `--interactive`.

`--format auto|delimited|json|ndjson` defaults to `auto`. Filename extensions
are considered before bounded content probing; an explicit format always wins.
Delimited-only options such as `--delimiter` imply delimited input under
`auto` and are rejected with an explicitly selected JSON format.

For structured formats, `--object-mode auto|record|entries` controls how a
selected object becomes rows. `record` keeps compatibility behavior and opens
the object as one row. `entries` opens each direct member as a row, preserving
source order; the member name is a synthetic first text column with canonical
identity `@key`. The same option is format-neutral so future structured input
adapters can use it too. It is not valid for arrays, scalars, delimited input,
or NDJSON row streams.

`auto`, the default, detects entries only when the bounded sample has at least
three members, every sampled value is an object, and at least 75 percent share
a direct child field with the same value kind. Detection examines at most 64
entries or 1 MiB, finishing the entry that crosses the byte bound. Use explicit
`record` or `entries` for reproducible scripts and saved views. Improvements to
future default detection do not override an explicit mode.

`--json-path` uses RFC 6901 JSON Pointer, not JSONPath, and selection happens
before object-mode resolution. For example, `--json-path /hits/hits` selects
Elasticsearch search hits while ignoring response metadata. Selected arrays
remain rows. For NDJSON the pointer is resolved in each complete document and
the selected object or array remains that document's single row.

Nested objects are flattened to canonical row-relative pointers. Nested arrays
remain atomic JSON cells. Native null, boolean, integer, floating-point, and
text values remain distinct; notably, JSON `null` is not an empty string.

Use as the pager for MySQL by setting these options in `~/.my.cnf`:

```ini
pager=tabview -d '\t' --quoting QUOTE_NONE -
silent
```

The Rust rewrite supports the `tabview` CLI only. The former Python import API
(`import tabview` and `tabview.view(...)`) is not part of the supported surface.

## Color Themes

Tabview loads theme settings from `$XDG_CONFIG_HOME/tabview/config.yml`, or
`~/.config/tabview/config.yml` when `XDG_CONFIG_HOME` is unset:

```yaml
theme: cmdzro
```

Theme files live in `tabview/themes/*.yml` or `tabview/themes/*.yaml` under the
same config directory. If both `name.yml` and `name.yaml` exist, `.yml` wins.
If no theme is configured, tabview uses the built-in `cmdzro` theme based on
`~/.config/nvim/colors/cmdzro.vim`: neutral gray text, blue reserved for UI
surfaces, yellow reserved for search and UI emphasis, and red reserved for
errors or unhealthy states.

Theme colors accept 16-color names, 256-color palette values, and 32-bit hex:

```yaml
name: ops-dark
mode: auto # auto, ansi16, ansi256, hex32, or truecolor

palette:
  text: "#AFAFAFFF"
  gray: gray
  muted: palette(240)
  ui_blue: palette(19)
  blue: blue
  dark_blue: palette(19)
  cyan: cyan
  dark_cyan: dark-cyan
  green: dark-green
  magenta: magenta
  yellow: yellow
  error: dark-red
  teal: "#25A39AFF"

identifiers:
  colors: [bright-green, magenta, cyan, white]

styles:
  table:
    location:
      fg: gray
      bg: black
    current_cell:
      fg: cyan
      bg: dark_blue
    divider:
      fg: gray
    header:
      fg: dark_cyan
      modifiers: [bold]
    header_selected:
      fg: cyan
      modifiers: [bold]
    header_glyph:
      fg: muted
    cell:
      fg: text
    selected:
      fg: text
      bg: dark_blue
    hidden_marker:
      fg: muted
  popup:
    background:
      fg: text
      bg: dark_blue
    border:
      fg: cyan
      bg: dark_blue
    title:
      fg: gray
      bg: dark_blue
    body:
      fg: text
      bg: dark_blue
    disabled:
      fg: muted
      bg: dark_blue
    active:
      fg: gray
      bg: dark_blue
    action:
      fg: cyan
      bg: dark_blue
    option_selected:
      fg: cyan
      bg: dark_blue
  search:
    highlight:
      fg: yellow
      modifiers: [underline]
  message:
    footer:
      fg: yellow
      bg: ui_blue
```

Named 16-color values use tabview's built-in cmdzro base palette; in truecolor
mode they resolve to those RGB values, while `mode: ansi16` emits ANSI colors
for the terminal palette.

See `sample/config/themes/cmdzro.yml` for a complete theme file. The
theme schema is shipped at `schemas/theme.schema.json`.

## Saved Views

By default, tabview loads user-defined YAML views
from `$XDG_CONFIG_HOME/tabview/views`, or `~/.config/tabview/views` when
`XDG_CONFIG_HOME` is unset. This POSIX-style path is used on every platform,
including macOS. Files ending in `.yml` and `.yaml` are accepted. If both
`name.yml` and `name.yaml` exist, `.yml` wins and a footer warning is shown.

Views match the opened input basename only. Filename entries are classified as
exact strings, globs containing `*`, `?`, or `[`, or regexes that start with
`^` or end with `$`. Exact matches win before globs, then regexes. Use
`--view <name>` to force a view by file stem, or `--no-view` to disable loading
and saving for that run.

Saved views can define sparse per-column state:

```yaml
name: cat-shards
filenames:
  - cat_shards.txt
format: delimited
schema_scan: default
nulls: last
columns:
  shard:
    type: integer
    width: header
    align: left
    nulls: first
  "*count":
    type: integer
    format: locale
    width: content
  segment:
    type: text
    visible: false
sort:
  - column: shard
    direction: asc
    kind: numeric
filters:
  - column: "*count"
    action: in
    kind: numeric
    condition: ">0"
```

A keyed-object view can pin its row shape and address the synthetic key column
independently of its display label:

```yaml
name: repositories
filenames: [repositories.json]
format: json
object_mode: entries
columns:
  "@key":
    label: Repository
```

Column keys match headers case-insensitively. Exact keys win over wildcard
keys; wildcard ties use the most literal characters, then lexical order.
Supported type aliases are `string`, `text`, `date`, `ip`, `number`, `float`,
`integer`, `semver`, `boolean`, `char`, `bit`, and `word`. Formats include
`plain`, `locale`, `mask`, `uppercase`, `lowercase`, `char`, `bit`, and `word`.
Number masks support `0`, `0.00`, `#,##0`, and `#,##0.00` forms. `locale`
uses the system POSIX locale with `en_US` fallback, or a top-level `locale`.
Headers are prefixed first with sort state, then filter state: `▲` for
ascending sort, `▼` for descending sort, `+` for filter-in, `-` for filter-out,
and `±` for multiple filters. Truncation applies after those prefix markers.

Source options are selected before the table opens. Saved views accept
`format`, `json_path`, `object_mode`, and `schema_scan`; precedence is explicit
CLI options, then the selected saved view, then defaults. Supplying
`--schema-scan default` therefore overrides a saved `schema_scan: full` for one
invocation. When a view is written for an object table, tabview saves the
resolved explicit `object_mode` (`record` or `entries`) so later detector
improvements do not change that view's shape. Non-object tables omit it.

Structured column configuration should use exact, case-sensitive canonical
JSON Pointers such as `/_source/user/email`; keyed-object member names use
`@key`, regardless of whether its display label is `name` or `_key`. An
unambiguous compact display label is accepted as a fallback. A column can set
`label` without changing its canonical identity or raw data. Top-level and
per-column `nulls: first|last`
control direction-independent sort placement, with the column policy winning
over the view policy and `last` as the built-in default.

## Large Files and Schema Discovery

Large seekable inputs are opened with incremental logical-row indexing. CSV
offsets come from the CSV parser, so quoted multiline records remain one row.
Navigation requests additional bounded ranges; viewport rendering, current-cell
popups, and yanks do not require a full-table clone.

JSON schema discovery examines up to 100 MiB of selected logical-row payload by
default and finishes the row crossing that boundary. A schema stopped at the
bound is provisional: newly encountered canonical paths append on the right,
earlier rows receive nulls, and existing labels and order remain fixed. Use
`--schema-scan full` when every column and inferred source type must be known
before the first data frame.

Exact sorting, filtering, maximum-width calculation, full auto-range profiling,
and similar whole-dataset operations may need to index or materialize the
selected table. Their cost grows with the complete source even when initial
opening was bounded. Stdin and encodings that cannot safely use byte offsets
use materialized storage.

Columns can also define ordered conditional color rules. The first matching
rule wins, and colors affect only cell styling; raw values, formatted values,
sorting, filtering, search, copy, and popups are unchanged.

```yaml
columns:
  active:
    type: boolean
    colors:
      - match:
          true: green
          false: muted
  prirep:
    type: string
    colors:
      - match:
          p: darkgreen
          r: blue
  used_percent:
    type: number
    colors:
      - range:
          "<10": red
          ">=90": red
      - gradient:
          mode: auto
          steps: 8
          colors: [green, yellow]
  latency_ms:
    type: number
    colors:
      - gradient:
          mode: fixed
          stops:
            0: green
            100: yellow
            500: red
  ip_address:
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

The `identifiers` rule is for string-like discrete values. It assigns each
unique rendered value in the column, such as an IP address or host name, to a
stable generated color. `colors: auto` uses the active theme's
`[identifiers].colors` families; a view can override those families with a
color array. Each family generates 16 dark-to-light shades, and identifiers
cycle across families before advancing shades. The darkest generated shade is
kept at the ANSI dark/dim foreground equivalent for that family rather than
near-black.

Press `v` to inspect the current generated YAML. Press `s` to save it to the
loaded view file, or to a placeholder file named from the current input with
only the last extension replaced by `.yml`. Existing files ask for `y`/`n`
confirmation. Saves are atomic and create the views directory as needed.

The schema for editor validation is shipped at `schemas/view.schema.json`.
See `sample/cat-shards.view.yml` for a complete example.

## Development

```sh
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## Keybindings

| Key | Action |
| --- | --- |
| `F1`, `?` | Show keybindings. |
| Cursor keys, `h`, `j`, `k`, `l` | Move the highlighted cell, scrolling if required. |
| `q`, `Q` | Quit. |
| `Home`, `^`, `Ctrl-a` | Move to the start of this row. |
| `End`, `$`, `Ctrl-e` | Move to the end of this row. |
| <code>[num]&#124;</code> | Go to column `num`, or the first column when `num` is omitted. |
| `PgUp`, `PgDn`, `J`, `K` | Move a page up or down. |
| `H`, `L` | Move a page left or right. |
| `g` | Go to the top of the current column. |
| `[num]G` | Go to row `num`, or the bottom of the current column when `num` is omitted. |
| `Ctrl-g` | Show file and data information. |
| `Insert`, `m` | Mark the current cell. |
| `Delete`, `'` | Return to the marked cell, if any. |
| `Enter` | View full cell contents in a popup. |
| `/` | Search. |
| `i` | Edit the current column view configuration, sort state, and filter action. |
| `f`, `F` | Filter in or filter out rows by the current column. `Tab` cycles text, regex, and numeric modes; submitting an empty condition clears filters for the current column. |
| `n` | Go to the next search result. |
| `N` | Go to the previous search result. |
| `t` | Toggle fixed header row. |
| `<`, `>` | Decrease or increase all column widths. |
| `,`, `.` | Decrease or increase the current column width. |
| `-`, `+` | Decrease or increase the column gap. |
| `s`, `S` | Sort the current column lexically, ascending or descending. |
| `a`, `A` | Sort the current column naturally, ascending or descending. |
| `#`, `@` | Sort the current column numerically, ascending or descending. |
| `r` | Reload file or input data and reset sort order. |
| `y` | Yank the rendered current cell to the clipboard when clipboard support is enabled. |
| `Y` | Yank the raw current cell to the clipboard when clipboard support is enabled. |
| `v` | Show the saved view modal when saved views are enabled. |
| `[num]z` | Toggle variable column width mode between `mode` and `max`, or set all columns to width `num`. |
| `[num]Z` | Maximize the current column, or set the current column to width `num`. |
| `[num]chh`, `[num]chl` | Hide visible columns to the left or right of the current column. |
| `chj`, `chk` | Hide the current column. |
| `[num]cHh`, `[num]cHl` | Show adjacent hidden columns to the left or right. |
| `csk`, `csj`, `csx` | Sort the current column ascending, sort descending, or clear its sort key. |
| `[num][` | Skip to the previous row value change. |
| `[num]]` | Skip to the next row value change. |
| `[num]{` | Skip to the previous column value change. |
| `[num]}` | Skip to the next column value change. |
