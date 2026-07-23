## Why

Tabview currently always enters an interactive terminal UI, so its format detection, saved views, column formatting, sorting, filtering, and table layout cannot be reused in pipelines or scripts. A non-interactive output path would make Tabview a convenient converter from CSV, JSON, NDJSON, and future table sources into readable fixed-width text.

## What Changes

- Retain automatic runtime selection when neither axis is specified: use the TUI when stdout is a terminal and emit a plain-text table when stdout is redirected or piped.
- Add composable CLI axes: `--interactive`/`-i` selects the TUI runtime, while `--output <format>`/`-o <format>` selects serialization. `-o table` renders immediately, `-i` is view-only, and `-i -o table` interactively transforms then writes the final view on normal quit. Future adapters add format values such as `csv` or `markdown` without being confused with runtime modes.
- Separate terminal UI I/O from data stdin/stdout so piped input can continue feeding the table while interactive mode uses the controlling terminal and reserves stdout for an optional transformed result.
- Apply the same source options and automatically selected or explicitly named saved view before rendering non-interactive output.
- Introduce a source-neutral batch-output adapter contract and implement fixed-width `table` as its first adapter, keeping lifecycle, preparation, diagnostics, and pipe handling shared for future formats.
- Render the complete logical result using configured labels, visible columns, order, formats, widths, alignment, filters, sorts, and header visibility without terminal viewport clipping or TUI chrome.
- Keep batch output unconstrained by terminal or aggregate row width by default; honor explicit per-column width configuration and leave wrapping, truncation, paging, or reflow to downstream consumers.
- Define deterministic fixed-width output, Unicode display-width padding/clipping, line termination, empty-result behavior, and separation of stdout data from stderr diagnostics.
- Add `--color auto|always|never`; non-interactive `auto` resolves to no ANSI color, so colored table output is opt-in with `always`.
- Handle downstream pipe closure cleanly, avoid terminal side effects in batch mode, and restore the terminal before an interactive session serializes its final view.
- Document CSV/JSON/NDJSON-to-text examples and saved-view-driven scripted output.

## Capabilities

### New Capabilities

- `non-interactive-output`: Runtime/output-format selection, composable interactive-transform export, complete fixed-width table rendering, color policy, stream/error behavior, and non-interactive execution semantics.

### Modified Capabilities

- `cli-compatibility`: Add explicit output and color mode arguments while making redirected stdout select table output by default.
- `saved-views`: Apply selected view configuration consistently to non-interactive table output.

## Impact

- Affects CLI/config parsing, startup and terminal-session branching, controlling-terminal I/O, stdin ingestion, source opening, saved-view and live-view application, query execution, width/profile calculation, a modular batch-output adapter boundary, the first stream-oriented text adapter, theme-to-ANSI conversion, error handling, documentation, and integration/golden tests.
- Reuses the format-aware store architecture from `large-file-store` and should remain compatible with `keyed-json-objects` because non-interactive rendering consumes the resulting table model rather than source-specific rows.
- Adds no required external dependency; ANSI output may reuse Ratatui/crossterm style data while plain output remains raw text.
