## Why

Many JSON APIs encode tables as objects whose keys identify records and whose values contain similarly shaped child objects. Tabview currently flattens such a selected object into one extremely wide row, making common Elasticsearch diagnostic files such as repositories, pipelines, index settings, aliases, mappings, and node maps difficult to inspect.

## What Changes

- Add conservative automatic detection of keyed JSON objects after JSON Pointer selection, based on entry count, object-valued children, and shared child schema.
- Add a format-neutral `auto`, `record`, and `entries` object-mode source option so users can accept inference, preserve single-record interpretation, or force selected object/map entries to become rows. JSON is the first adapter to implement the shared option; future YAML, TOON, and other structured adapters can reuse it without adding format-specific flags.
- Project each keyed entry into a row whose first column contains the object key and whose remaining columns are the recursively flattened child properties.
- Give the synthetic key column stable identity and the default label `name`, with a non-conflicting fallback when a child property already uses that label.
- Reject duplicate direct member keys rather than silently overwriting them or exposing ambiguous keyed rows.
- Preserve array-table behavior and the existing one-row-per-document NDJSON contract.
- Support keyed objects in both materialized and incremental JSON stores so large maps remain bounded during schema discovery, initial rendering, navigation, and reload.
- Surface the inferred object mode and the available override in table/source information.
- Document explicit `record` and `entries` modes as the way to pin table shape for reproducible invocations.
- **BREAKING**: In `auto` mode, a selected JSON object that meets the conservative keyed-object criteria will render as multiple rows instead of the previous single flattened row. `record` mode restores the previous interpretation.

## Capabilities

### New Capabilities

- `keyed-objects`: Detection, explicit interpretation modes, key-column identity, row projection, schema behavior, and incremental access for keyed object tables.

### Modified Capabilities

- `cli-compatibility`: Add the format-neutral `--object-mode` source option with validated `auto`, `record`, and `entries` values.
- `saved-views`: Allow saved views to select and persist `object_mode` before opening an object-capable structured source.

## Impact

- Affects shared source options and precedence, adapter capability validation, JSON selection and streaming visitors, schema discovery, materialized and lazy JSON stores, table definitions, format-neutral column identity/display labels, opened-source metadata, saved-view parsing/schema/serialization, CLI help, fixtures, and render/integration tests.
- Depends on the format-aware table/store and JSON ingestion architecture introduced by `large-file-store`.
- Adds no external dependency, but requires streaming keyed-map entry indexing rather than materializing a large selected object as one value.
