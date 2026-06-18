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
- Full-cell popup with `Enter`.
- Optional clipboard support for yanking the current cell.
- In-place reload when data changes.
- Built-in keybinding help with `F1` or `?`.
- Example screenshots in the `screenshots/` directory.

## Requirements

- Rust toolchain for installation with Cargo.
- Optional clipboard support can be enabled with the `clipboard` Cargo feature.

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
| `y` | Yank the current cell to the clipboard when clipboard support is enabled. |
| `[num]c` | Toggle variable column width mode between `mode` and `max`, or set all columns to width `num`. |
| `[num]C` | Maximize the current column, or set the current column to width `num`. |
| `[num][` | Skip to the previous row value change. |
| `[num]]` | Skip to the next row value change. |
| `[num]{` | Skip to the previous column value change. |
| `[num]}` | Skip to the next column value change. |
