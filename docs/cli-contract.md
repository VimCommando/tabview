---
type: Guide
title: CLI output and compatibility
description: Output schemas, exit codes, stream behavior, and compatibility rules.
generated: { by: codex/gpt-6, at: 2026-09-07T02:19:31Z }
---

# CLI output and compatibility

The default remains the interactive viewer on a terminal and fixed-width table
output when stdout is redirected. Explicit `--output` selects batch output;
combining it with `--interactive` exports the final view after a normal quit.

## JSON and JSONL

`--output json` emits one UTF-8 JSON document, followed by a newline:

```json
{"columns":["Name","Count"],"rows":[["alpha","2"],["beta","10"]]}
```

`--output jsonl` emits one JSON object per row, each followed by a newline:

```json
{"columns":["Name","Count"],"values":["alpha","2"]}
{"columns":["Name","Count"],"values":["beta","10"]}
```

Cells are displayed strings, not a lossless export of native source types.
Saved-view formatting, filtering, sorting, and hidden columns affect the result.
Width clipping, padding, control-character replacement, and ANSI styling do not.
JSON escapes embedded newlines and control characters. Ordered arrays retain
columns with duplicate names. `columns` is empty when the header is hidden or
absent; rows still contain positional values. Empty JSON results have an empty
`rows` array; empty JSONL results contain zero records. Column labels are repeated
in each JSONL record so each line describes itself.

The schemas are [JSON](../schemas/output.schema.json) and
[JSONL record](../schemas/output-record.schema.json). Additive fields may appear
in future versions; consumers should ignore unknown fields. Existing field
meanings and string cell types are stable within a major release. A native typed
export would require a separately documented format rather than changing these
cells silently. TOON output is deferred until a measured consumer benefit exists.

Both formats reject `--color always`. Data goes to stdout and diagnostics go to
stderr. Structured batch output never prompts for source selection.

## Exit codes and partial output

| Code | Meaning |
|---|---|
| 0 | Success, help, version, normal interactive quit, or a consumer closing its pipe. |
| 1 | Source, configuration, runtime, output, or incompatible runtime-option error. |
| 2 | Command-line syntax or value rejected by the argument parser. |

OS signal termination retains OS-defined status. Cancelling an interactive
operation does not export an unfinished result. Broken pipes during writing or
flush count as success. Other write failures return 1 and may leave partial bytes.
A failure during source preparation produces no output. A process killed during
serialization can leave truncated JSON or JSONL; the output is not a transaction.

All serializers wait for complete source preparation and late schema discovery
before writing. Stdin waits for EOF and can materialize the entire input. Some
sorts, filters, and exports can require full materialization even when initial
viewing was incremental. JSONL framing does not make ingestion bounded-memory.
Remote source limits and timeouts remain enforced.

## Writes and compatibility

The viewer reads source files and remote data. Saving a view writes local config
and prompts before overwriting it. Shell redirection opens its destination before
Tview runs; choose a different path from the input. A dry-run mode for source
mutations is not applicable because the viewer does not offer them.

Stable CLI meanings follow semantic versioning after the 2.0 prerelease series.
Prerelease changes still need explicit migration notes. Preserve existing defaults
until a planned major compatibility change. See [migration](migration.md).
