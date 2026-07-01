# Tabview

[![Crates.io](https://img.shields.io/crates/v/tabview.svg)](https://crates.io/crates/tabview)
[![License](https://img.shields.io/crates/l/tabview.svg)](LICENSE.txt)
[![Sourcegraph](https://sourcegraph.com/github.com/TabViewer/tabview/-/badge.svg)](https://sourcegraph.com/github.com/Tabviewer/tabview)

View CSV and delimited text files in a spreadsheet-like terminal interface.

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
- Vim-like navigation, including `h`, `j`, `k`, `l`, `g`, `G`, marks, and
  numeric prefixes such as `12G`.
- Persistent header row toggling.
- Lexical, natural, and numeric sorting by the current column.
- Dynamic column width and gap adjustment.
- Full-text incremental search with `n` and `p` result navigation.
- Current-column filter-in and filter-out with text, regex, and numeric modes.
- Full-cell popup with `Enter`.
- Optional clipboard support for yanking the current cell.
- In-place reload when data changes.
- Built-in keybinding help with `F1` or `?`.
- Example screenshots in the `screenshots/` directory.

## Requirements

- Rust toolchain for installation with Cargo.
- Optional clipboard support can be enabled with the `clipboard` Cargo feature.
- Optional saved views can be enabled with the `saved-views` Cargo feature.

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

Build with saved view support:

```sh
cargo install tabview --features saved-views
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
```

Read from standard input:

```sh
cat data.csv | tabview -
```

Use as the pager for MySQL by setting these options in `~/.my.cnf`:

```ini
pager=tabview -d '\t' --quoting QUOTE_NONE -
silent
```

The Rust rewrite supports the `tabview` CLI only. The former Python import API
(`import tabview` and `tabview.view(...)`) is not part of the supported surface.

## Saved Views

When built with `--features saved-views`, tabview loads user-defined YAML views
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
columns:
  shard:
    type: integer
    width: header
    align: left
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
| `p` | Go to the previous search result. |
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
