## Context

The startup path currently parses configuration, enters a Crossterm raw/alternate-screen terminal session, draws a loading footer, opens the source, constructs `TableView`, applies a saved view, and then runs a draw/event loop. Ratatui rendering is viewport-oriented and includes cursor, location, divider, hidden-column markers, selection styles, footer messages, and clipping to the terminal area.

The underlying view already owns the source-neutral concerns needed for batch output: typed/rendered values, source-defined headers, saved labels and formats, visible-column mapping, alignment, widths, filters, sorts, conditional colors, and incremental table-store access. Non-interactive output needs a startup branch and a modular output-adapter boundary that reuse this model without simulating a terminal screen. Fixed-width text is the first adapter, but the boundary must allow later formats such as Markdown without another startup refactor.

## Goals / Non-Goals

**Goals:**

- Select table output automatically for redirected or piped stdout.
- Allow explicit, deterministic, and independent selection of interactive runtime, output format, and color behavior.
- Allow interactive sessions with an output format to act as transforms that emit their final live view to stdout on normal quit.
- Apply the same source and saved-view semantics in interactive and batch modes.
- Emit a stable, complete, readable fixed-width table suitable for files and Unix pipelines.
- Keep default batch output independent of terminal dimensions and free of an aggregate width cap or automatic reflow.
- Keep stdout data-only, handle broken pipes conventionally, and avoid all terminal side effects in table mode.
- Work across delimited, JSON, NDJSON, keyed-object, incremental, and future table stores.
- Keep `--output` exclusively format-oriented and make the output-adapter path extensible so future document formats can reuse common source/view preparation, diagnostics, and stream handling.

**Non-Goals:**

- Implementing CSV, JSON, Markdown, HTML, or database output adapters in this change; this change only establishes the adapter contract and implements fixed-width `table`.
- Adding an output-file option; normal shell redirection remains the destination mechanism.
- Reproducing TUI chrome, cursor/selection state, popups, search highlighting, or viewport clipping.
- Adding a total-output-width, wrapping, or reflow option; downstream consumers are responsible for those presentation policies.
- Streaming the first row before widths and schema are stable; deterministic alignment takes precedence over single-pass latency.
- Persisting interactive changes made during a prior process unless they were saved in a view.
- Replacing an input file in place or providing atomic file replacement; callers must redirect to a distinct path and replace the original only after success.

## Decisions

### 1. Compose runtime mode and output format before terminal construction

Treat runtime mode and serialization format as independent CLI axes. Parse `--interactive`/`-i` as a boolean runtime request and `--output <format>`/`-o <format>` as `Option<OutputFormat>`. `OutputFormat` initially has only `Table`; future variants such as `Csv` and `Markdown` extend only the format enum. Resolve both axes before terminal construction into `ExecutionMode::{Interactive { emit_on_exit: Option<OutputFormat> }, Batch(OutputFormat)}`.

Resolution forms a small composition matrix:

- neither option, terminal stdout → `Interactive { emit_on_exit: None }`;
- neither option, non-terminal stdout → `Batch(Table)`;
- `-i` without `-o` → `Interactive { emit_on_exit: None }` regardless of stdout;
- `-o <format>` without `-i` → `Batch(format)` regardless of stdout;
- `-i -o <format>` → `Interactive { emit_on_exit: Some(format) }` regardless of stdout.

This preserves existing automatic behavior while making intent composable. `-i` alone is an explicit view-only session. `-o table` is a direct fixed-width conversion. `-i -o table` freezes the final live view after normal quit and sends it through the fixed-width adapter. Once added, `-i -o csv` or `-i -o markdown` will use the same runtime lifecycle with a different adapter.

Refactor startup into shared configuration/source/view preparation surrounded by mode-specific lifecycle:

```text
parse CLI + resolve execution/export intent
                    |
             select source options
                    |
        +-----------+-----------+
        |                       |
   INTERACTIVE                BATCH
open UI terminal                |
start/drain source               |
construct + edit live view       |
normal quit                      |
restore UI terminal              |
        |                        |
 emit_on_exit?                   |
        +-----------+------------+
                    |
        complete + render stdout
```

The interactive branch preserves the requirement to enter the terminal before potentially slow opening work. Automatic interactive mode stops after terminal restoration. Explicit TUI mode continues into final-output preparation only after restoration. The batch branch reports only diagnostics to stderr and never writes loading/progress text to stdout.

Alternative considered: render a very large virtual Ratatui `Buffer` and print it. That retains unwanted screen chrome, requires arbitrary dimensions, loses streaming writes, and conflates terminal viewport behavior with a document format.

Alternative considered: include `auto` and `tui` in the `--output` value enum. Those are runtime policies rather than serialization formats, prevent natural composition such as `tabview -i -o csv`, and force special cases when an interactive session should also emit data. Dedicated `--interactive` plus format-only `--output` keeps each option single-purpose.

### 2. Separate UI channels from data channels

Treat stdin/stdout as data channels and the selected terminal device as the interactive UI channel. When interactive input comes from a file and stdout is a terminal, existing standard terminal handles may be reused. When stdin is the table source or stdout is redirected, open the controlling terminal for event input and screen output (`/dev/tty` on Unix and the corresponding console handles on Windows). Validate this terminal before consuming piped input; if none is available, `--interactive` fails with stderr diagnostics and emits no stdout.

For non-seekable stdin, read enough data to establish the provisional schema and display the initial table, then continue draining/materializing the pipe into the incremental backing store while the event loop runs. This prevents a finite producer from deadlocking on a full pipe and allows late rows/schema to appear interactively. A normal quit that requests final output completes ingestion to EOF, applies any late schema, and prepares the final current result before writing stdout. Cancellation, terminal failure, ingestion failure, or an abnormal exit produces no final table.

The UI never writes control sequences, loading text, or diagnostics to redirected stdout. Restore raw mode and the alternate screen before invoking the final output adapter, so stdout contains only the final table when redirected.

The shell opens redirection targets before starting the pipeline. Therefore commands such as `cat file.csv | tabview -i -o table - > file.csv` or `tabview -i -o table file.csv > file.csv` are unsafe: they may truncate the input before Tabview can read it. Documentation uses a distinct destination, for example `cat file.csv | tabview -i -o table - > edited.txt`. Because this change's table adapter emits fixed-width text rather than CSV, callers must not replace the original CSV with that output. A caller-controlled atomic rename is appropriate only when a future selected adapter preserves the intended destination format, such as `-i -o csv` once CSV output exists.

### 3. Use one configured-view preparation path

Extract the common work that opens an input, constructs `TableView`, applies source options, selects/applies saved views, and resolves warnings. The result is shared by all branches. TUI-only start position applies to the interactive cursor, but does not restrict an interactive final export.

When interactive mode has an output format, freeze the live view state after normal quit, including current column visibility/order, labels, formats, alignment and configured widths, header visibility, filters, sort order, and null placement. Exclude cursor/viewport position, selection, search highlights, popups, and other ephemeral screen decoration. Run the same complete-output preparation used by batch mode against this frozen state so the export includes every matching row, including rows ingested after the initial schema/view was displayed.

For table output, add a preparation operation that indexes/scans the active source through completion, applies late schema deltas and pending saved-column configuration, executes saved filters/sorts with canonical semantics, resolves complete conditional-color profiles where required, and freezes visible columns, alignments, and widths. Failures during this phase occur before stdout emission whenever possible.

Alternative considered: duplicate saved-view application in a new exporter. Duplication would quickly diverge on JSON source options, late columns, typed sorting, null placement, and formatting.

### 4. Let adapters declare preparation needs

Common preparation produces a source-neutral configured result after source options, saved views, late schema, filters, and sorts have been applied. Each output adapter declares additional `OutputRequirements`, such as complete traversal, stable display widths, rendered text, or conditional style profiles. The batch driver satisfies those requirements before the adapter writes any bytes.

The fixed-width table adapter requires stable widths before the header and first row are written. It therefore requests complete logical traversal and width/profile reduction, then emits the header and rows through a buffered writer. The adapter applies no terminal-derived or aggregate output-width limit and never wraps or reflows rows. Explicitly configured per-column fixed or maximum widths remain authoritative; without such a cap, a column expands to the widest normalized header or rendered value in the complete result. A future Markdown adapter can build its own layout from the same prepared result without inheriting fixed-width padding or ANSI behavior.

This deliberately allows very wide rows. Shell tools, pagers, files, and other consumers can truncate, wrap, scroll, or reflow according to their own environment without Tabview guessing a downstream viewport.

The implementation should prefer store scan/fold and repeatable indexed row access over cloning an extra `Vec<Vec<String>>`. Query fallback may already materialize a derived result, but the text renderer does not require another full copy. Non-seekable input is already materialized by its adapter where repeatable access is required.

Alternative considered: grow widths as later rows arrive. That produces misaligned output whose column positions change mid-stream and makes a saved fixed-width view non-deterministic.

### 5. Build a modular output-adapter boundary and fixed-width adapter

Add an output module with an `OutputAdapter` contract selected by an `OutputFormat` factory. An adapter receives an immutable source-neutral `PreparedOutput`, adapter-specific validated options, and a `Write` sink. It reports preparation requirements and writes only output-format bytes; it does not open sources, apply saved views, choose a terminal lifecycle, print diagnostics, or decide process exit status.

The initial `FixedWidthTableAdapter` consumes the prepared projection rather than a Ratatui area. It owns its layout rules and emits:

- an optional header line using resolved visible labels;
- every rendered logical row in active result order;
- configured gaps as literal spaces;
- left/right padding and clipping by Unicode display width;
- no divider, borders, cursor glyphs, hidden markers, footer, or trailing spaces.

Before measuring, normalize embedded controls to visible escapes (`\n`, `\r`, `\t`, `\e`, and a stable escaped form for other controls) so each logical row remains one physical line. Clipping walks valid UTF-8 scalar values and tracks display width; ANSI is added only after clipped/padded content is known.

Factor shared cell layout helpers from the TUI renderer where doing so preserves existing behavior, but do not make any output adapter depend on `Rect` or `Buffer`. Keep fixed-width clipping, padding, separators, and ANSI conversion inside the table adapter so a future Markdown adapter can instead own pipe escaping, delimiter rows, and Markdown alignment markers.

The adapter factory performs exhaustive `OutputFormat` dispatch. Direct batch mode and interactive final export call the same factory and stdout driver. Adding a format therefore consists of adding one format enum value, one adapter implementation, its option validation and tests, without changing source opening, saved-view application, terminal branching, or stream handling.

Alternative considered: delimit cells with tabs. Tabs are compact but do not preserve saved widths/alignment and render differently across consumers.

### 6. Make color explicitly opt-in for table bytes

Add `ColorOutput::{Auto, Always, Never}`. The interactive screen retains existing terminal/theme resolution independently of final serialization. The fixed-width table adapter declares color support; for direct or post-interactive output, `Auto` resolves to `Never` and only `Always` writes ANSI. Adapter capabilities are validated before preparation or output. A future adapter that does not support ANSI, such as Markdown, rejects an incompatible explicit `--color always` rather than silently corrupting its format.

Colored table output uses the resolved theme's header/cell styles and existing conditional cell-color context, but omits cursor/selected/search overlays. Convert Ratatui foreground/background/modifiers to ANSI sequences, emit styles around cell content and padding, and reset before separators/newlines so style cannot leak. Width calculations operate on unstyled strings.

Alternative considered: color whenever stdout is a terminal. Explicit `--output table` to a terminal is still intended to be capturable/reproducible; the user requested color as opt-in, so table output remains plain under color `auto`.

### 7. Define Unix-friendly I/O and failure behavior

Use one output driver for direct and post-interactive export, with locked buffered stdout for adapter bytes and stderr for warnings/errors. A source, saved-view, traversal, adapter-preparation, or profiling failure before emission leaves stdout empty and returns failure. A non-`BrokenPipe` write failure returns failure. `BrokenPipe` means the downstream consumer intentionally stopped, so rendering terminates successfully without a diagnostic. These policies stay outside adapters so every future format and lifecycle has the same pipeline behavior.

The renderer writes one `\n` after each emitted line and no extra blank line. An empty headerless result writes nothing; an empty result with a visible header writes the header only.

Alternative considered: print warnings as footer lines. That corrupts redirected data and makes downstream text processing unreliable.

### 8. Keep input sources and output formats independent

Output adapters receive the completed view projection and never branch on CSV, JSON, NDJSON, keyed-object, or future input adapter types. Input-format work remains entirely in source opening and table construction; output-format work remains behind `OutputAdapter`. This forms two independent axes: any supported input source can feed any compatible output adapter.

## Risks / Trade-offs

- **[Automatic non-interactive behavior changes existing redirected invocations]** → Document stdout detection; no flags with redirection selects immediate table output, while `-i` independently opts into controlling-terminal interaction and `-o` independently requests final serialization.
- **[Piped stdin can block while the user is interacting]** → Drain/materialize stdin concurrently after provisional schema detection and complete ingestion before final export.
- **[Input and output redirection can name the same file]** → Document that the shell truncates the target before Tabview starts; require a distinct output path, identify the fixed-width result as text, and permit caller-controlled atomic replacement only with a format-appropriate adapter.
- **[Exact width calculation delays first output]** → Treat stable alignment as the contract, reuse scan/fold reductions, and avoid unnecessary row clones.
- **[Very large results require two logical passes]** → Use indexed stores and buffered output; the command already must traverse all rows to emit them.
- **[Unconstrained cells can produce extremely wide rows]** → Preserve complete data by default, honor explicit per-column caps, and leave aggregate truncation, wrapping, paging, or reflow to the consumer.
- **[Late source mutation can still fail during the output pass]** → Validate generation/fingerprints on access, report non-broken-pipe failures, and avoid claiming atomic output after bytes have been written.
- **[ANSI conversion differs from Ratatui terminal rendering]** → Golden-test supported colors/modifiers and reset boundaries; keep plain output the default.
- **[A shared adapter contract can leak fixed-width assumptions]** → Keep widths, padding, ANSI, and separators in `FixedWidthTableAdapter`; expose source-neutral values, labels, ordering, alignment intent, and styles through `PreparedOutput`, and require format-specific layout plans.
- **[Control escaping changes rendered width]** → Normalize before profiling so clipping and alignment use the actual emitted representation.
- **[Saved filters/sorts can force expensive materialization]** → Reuse canonical operation semantics and document that full output necessarily performs complete work.
- **[Dependent active changes alter source/view behavior]** → Integrate `large-file-store` first and test this mode against `keyed-json-objects` once both are available.

## Migration Plan

1. Integrate the format-aware store/view baseline from `large-file-store`.
2. Add interactive/output/color CLI types and resolve their composition into automatic, batch, view-only, and interactive-transform lifecycles.
3. Separate terminal UI handles from stdin/stdout data handles and support continued stdin materialization during interaction.
4. Extract shared source/view preparation, live-view freezing, and requirement-driven output preparation APIs.
5. Add the output-adapter contract/factory and implement the fixed-width table adapter, then add opt-in ANSI conversion.
6. Connect direct and post-interactive output to shared stderr/broken-pipe handling, documentation, and end-to-end pipeline tests.

Rollback can disable automatic table selection while retaining ordinary automatic TUI behavior. Runtime and serialization remain independently reversible: `-i` selects a view-only TUI, `-o table` selects direct conversion, and combining them selects interactive transformation.
