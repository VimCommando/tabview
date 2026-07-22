## 1. Prerequisites and CLI Modes

- [ ] 1.1 Integrate or archive `large-file-store`, then verify this change's deltas against the resulting format-aware source, view, CLI, and saved-view specifications.
- [ ] 1.2 Add boolean `--interactive`/`-i`, format-only `--output`/`-o` as `Option<OutputFormat>`, resolved `ExecutionMode::{Interactive { emit_on_exit: Option<OutputFormat> }, Batch(OutputFormat)}`, initial `OutputFormat::Table`, and `ColorOutput::{Auto, Always, Never}` with documented defaults.
- [ ] 1.3 Resolve neither option from stdout terminal detection, `-i` to interactive runtime, `-o <format>` to serialization, and their combination to interactive final export through the selected adapter.
- [ ] 1.4 Add CLI tests for valid/invalid output formats and color values, short/long flags, terminal/non-terminal defaults, view-only `-i`, direct `-o table`, and composed `-i -o table`.

## 2. Shared Startup and Lifecycle

- [ ] 2.1 Extract shared source-option resolution, source opening, `TableView` construction, saved-view application, theme loading, and warning collection from the interactive run path.
- [ ] 2.2 Add terminal-handle selection that separates UI input/output from data stdin/stdout, using the controlling terminal when stdin is the source or stdout is redirected and failing before input consumption when no terminal is available.
- [ ] 2.3 Branch into automatic/view-only interactive, interactive-transform, or shared batch lifecycle before constructing `TerminalSession`, keeping every batch adapter free of raw mode, alternate-screen, drawing, events, and clipboard access.
- [ ] 2.4 Preserve the TUI's pre-open loading frame, terminal restoration on failure, start-position behavior, and event loop; restore the terminal before explicit final export.
- [ ] 2.5 Drain/materialize non-seekable stdin after provisional schema detection while the TUI remains responsive, and complete finite input ingestion on normal quit before export.
- [ ] 2.6 Add lifecycle tests for controlling-terminal selection, missing terminals, piped stdin, redirected stdout cleanliness, concurrent ingestion, restoration-before-output, automatic no-export, explicit export, cancellation, and failures.

## 3. Complete Output Preparation

- [ ] 3.1 Add a complete-output preparation API that indexes or scans the active source/result through its end and applies schema deltas without depending on viewport navigation.
- [ ] 3.2 Apply pending structured saved-column configuration after late schema discovery and complete canonical saved filters/sorts before freezing output layout.
- [ ] 3.3 Define adapter `OutputRequirements` and satisfy requested complete traversal, rendered values, exact widths, alignments, visible order, header state, and style profiles before output begins; table preparation must not apply a terminal-derived or aggregate width cap.
- [ ] 3.4 Expose an immutable, source-neutral `PreparedOutput` that contains common configured projection data without fixed-width padding, separators, ANSI sequences, or other adapter-specific layout.
- [ ] 3.5 Freeze live state when interactive mode has an output format, including current column configuration, filters, sorts, and header state while excluding cursor, viewport, selection, search, popup, and other screen-only state.
- [ ] 3.6 Add preparation tests for incremental stores, late columns, pending saved settings, filters, sorts, live TUI modifications, screen-state exclusion, empty results, full widths, and failure-before-output behavior.

## 4. Plain Fixed-Width Renderer

- [ ] 4.1 Add an `OutputAdapter` contract, exhaustive `OutputFormat` factory, capability/option validation, and shared writer driver; implement an immutable fixed-width table layout plan as the first adapter.
- [ ] 4.2 Implement single-line control normalization for newline, carriage return, tab, escape, and other control characters before display-width profiling.
- [ ] 4.3 Implement valid-UTF-8 Unicode display-width clipping only for explicitly capped columns plus left/right padding shared where practical with existing TUI cell layout; do not wrap or reflow rows.
- [ ] 4.4 Implement plain header/data line emission with configured gaps, no TUI chrome or divider, no final-cell trailing spaces, and one newline per emitted line.
- [ ] 4.5 Implement header-only and zero-byte empty-result behavior plus hidden-header output.
- [ ] 4.6 Add golden/unit tests for labels, ordering, hidden columns, unconstrained automatic widths, explicit per-column caps, alignment, very wide rows, wide/combining Unicode, clipping, controls, gaps, line endings, and empty tables.

## 5. Opt-In ANSI Rendering

- [ ] 5.1 Declare color support as an adapter capability, resolve table color `Auto` to plain output, retain existing TUI automatic color behavior, and reject `Always` for future adapters that do not support ANSI.
- [ ] 5.2 Convert supported Ratatui theme foreground, background, and modifiers into ANSI start/reset sequences without affecting measured width.
- [ ] 5.3 Apply header, ordinary cell, and conditional cell styles while excluding cursor, selection, search, footer, and popup overlays.
- [ ] 5.4 Reset styles before separators and line endings and add golden tests for color modes, conditional colors, modifiers, padding, and escape leakage.

## 6. Stream and Error Contract

- [ ] 6.1 Write direct and post-interactive adapter bytes through one locked buffered-stdout driver and route source, theme, saved-view, terminal, ingestion, and adapter-validation warnings/errors exclusively to stderr.
- [ ] 6.2 Treat stdout `BrokenPipe` as clean early completion without a diagnostic while returning failure for other write errors.
- [ ] 6.3 Ensure source opening, complete traversal, query execution, and profiling errors occur before the first output line whenever possible and return nonzero.
- [ ] 6.4 Add writer-failure tests for broken pipes, non-broken write errors, stderr warnings, empty stdout on direct or post-interactive preparation failure, and successful flush.

## 7. Saved Views and Documentation

- [ ] 7.1 Apply automatic, forced, and disabled saved-view selection identically in table and TUI modes without duplicating configuration logic.
- [ ] 7.2 Verify non-interactive application of source options, labels, formats, visibility, order, widths, alignment, null placement, filters, sorts, header state, and late structured columns.
- [ ] 7.3 Document automatic stdout detection, `--interactive`/`-i`, `--output`/`-o`, their composition, controlling-terminal requirements, color, stdin/stdout pipelines, safe distinct-path redirection, fixed-width text versus source-format preservation, format-appropriate caller-controlled replacement, saved views, full-result costs, unconstrained default width and consumer-controlled reflow, input examples, and future adapters such as CSV/Markdown.

## 8. End-to-End Verification

- [ ] 8.1 Add CLI integration tests using terminal and non-terminal stdout for automatic view-only TUI, explicit `-i`, direct `-o table`, composed `-i -o table`, missing controlling terminal, unsupported formats, plain-by-default output, explicit ANSI output, and adapter dispatch.
- [ ] 8.2 Add pipeline tests for file input, incrementally drained stdin, interactive edits exported on quit, view-only no-export, early-closing consumers, stderr/UI separation, terminal restoration, start-position non-truncation, cancellation/failure no-export, and exit statuses.
- [ ] 8.3 Add source-neutral golden tests for delimited, JSON arrays, NDJSON, large incremental inputs, and keyed JSON objects when that change is available.
- [ ] 8.4 Add regressions proving interactive loading, rendering, navigation, saved views, themes, and terminal restoration remain unchanged.
- [ ] 8.5 Run formatting, linting, default-feature tests, no-default-feature tests, and release build verification.
