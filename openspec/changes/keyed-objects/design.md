## Context

The JSON adapter currently treats a selected array as a sequence of rows and a selected object as one recursively flattened row. Many Elasticsearch and other API responses instead use the selected object as an associative table: each direct member key is the record identity and each member value is a similarly shaped record. Flattening these maps as one record produces hundreds or thousands of key-prefixed columns and hides the useful row structure.

The completed `large-file-store` change provides format-aware source options, typed rows, stable source identity, JSON Pointer selection, bounded schema discovery, incremental array stores, schema deltas, and reload. This change builds on that architecture and must preserve bounded behavior for large keyed maps such as mappings, settings, pipelines, aliases, nodes, and repositories.

## Goals / Non-Goals

**Goals:**

- Detect high-confidence keyed object tables automatically after JSON Pointer selection.
- Provide explicit modes that make ambiguous object interpretation controllable and reproducible.
- Produce a stable first key column plus child-relative typed columns.
- Preserve source member order, schema evolution, saved-view matching, and reload behavior.
- Stream and index large selected maps without materializing the complete object.
- Make inferred behavior visible so a false positive has an obvious override.

**Non-Goals:**

- Inferring rows from arbitrary nested objects at more than the selected object's direct member level.
- Exploding arrays, joining nested tables, JSONPath, or general JSON transformations.
- Multiplying one NDJSON document into multiple rows.
- Perfect semantic classification of every structured object/map; explicit modes are the escape hatch for inherent ambiguity.
- Adding an interactive runtime command to reinterpret an already opened source; CLI and saved-view configuration are sufficient for this change.
- Adding YAML, TOON, or other new format parsers; this change defines a reusable object-mode contract and implements it for JSON.
- Changing stdin buffering or format resolution; buffered stdin schema resolution belongs to the `non-interactive-output` change.

## Decisions

### 1. Add one format-neutral source option with three modes

Introduce `ObjectMode::{Auto, Record, Entries}` in merged source options. CLI uses `--object-mode`; saved views use `object_mode`. Neither the type nor either external name is JSON-specific. Resolution follows existing precedence `explicit CLI > selected saved view > Auto` and occurs before opening the table.

Adapters declare whether their input model has a selected object/map to which the option can apply. The JSON adapter is the first consumer and applies it after JSON Pointer selection. Future YAML, TOON, or other structured adapters can apply the same resolved option after their own selection step. Row-stream formats with no single selected object, such as delimited input and NDJSON, reject explicit `record` or `entries` values rather than silently ignoring them.

Applicability is explicit:

| Selected input shape | `auto` | Explicit `record` or `entries` | Saved-view write |
|---|---|---|---|
| Object/map | Run detection | Force the requested shape | Write the resolved explicit mode |
| Array or scalar | Preserve existing behavior | Report an incompatible option | Omit `object_mode` |
| Row stream such as delimited or NDJSON | No-op | Report an incompatible option | Omit `object_mode` |

Because format `auto` is unresolved while source options are merged, adapter compatibility validation occurs after format resolution and, where necessary, after selection reveals the input shape. An incompatible CLI option is an opening error. An incompatible saved-view value produces the normal non-fatal saved-view warning and is not applied. This change does not make `--object-mode` imply a format or alter stdin buffering; stdin behavior is owned by `non-interactive-output`.

`Record` is the compatibility path. `Entries` forces direct object members into rows and supports object or scalar member values. `Auto` chooses between them using bounded evidence. Arrays bypass object interpretation, and NDJSON retains its one-row-per-document contract; explicit incompatible format combinations fail validation.

Alternative considered: add only a force flag such as `--object-keys-as-rows`. That avoids inference risk but leaves the common case undiscoverable across large diagnostic collections and cannot explicitly preserve record behavior when automatic inference is enabled.

### 2. Use a conservative, deterministic detection sample

Detection is defined over a selected structured object/map rather than over JSON syntax or an outer document. For the initial JSON adapter, it runs after JSON Pointer selection and samples at most 64 direct entries and at most 1 MiB of encoded entry data, finishing the entry crossing the byte bound. A map is classified as entries only when:

1. At least three entries were sampled.
2. Every sampled value is an object.
3. At least one direct child property appears with the same JSON value kind in `ceil(0.75 * sampled_entries)` children.

An explicitly present property whose value is `null` counts as present with JSON value kind `null`. A missing property does not count. This distinction preserves schema evidence carried by explicit nulls without treating absence as a value.

This detects the observed repositories map (`type` and `settings` in all entries), pipelines (`processors`), index settings (`settings`), and aliases (`aliases`) while avoiding scalar-bearing records and small section objects such as `persistent`/`transient` cluster settings. Classification is fixed for the opened generation. Later heterogeneous entries are still representable by the entries projector.

These thresholds define the initial default detector, not a permanently frozen compatibility contract. They may evolve as a broader corpus reveals better defaults. Compatibility instead comes from explicit configuration: CLI and saved-view `record` or `entries` values always bypass detection, and writing a saved view records the currently resolved explicit mode rather than `auto`. A saved view therefore keeps its chosen table shape across detector improvements unless the user explicitly changes or removes that setting.

Alternative considered: classify every object whose values are objects. It is easy to explain but misclassifies ordinary documents containing a few nested sections. Pairwise schema similarity was also considered, but the shared-field threshold is easier to test, explain, and keep deterministic.

### 3. Represent selected shape explicitly

Represent the shared post-selection shape with an internal model such as:

```text
SelectedTableShape
├── ArrayRows
├── ObjectRecord
└── ObjectEntries
```

Each object-capable adapter resolves its format-specific selection first, then determines shape from the effective mode and bounded detector. RFC 6901 JSON Pointer syntax is the canonical path-resolution and saved-column notation for structured values, but the shared path and shape implementation is adapter-neutral rather than tied to the JSON adapter. The JSON selection layer resolves its JSON Pointer before this step. Materialized and incremental paths consume the same resolved shape so inference does not diverge by file size.

Object entries project as `(member_key, member_value)`. Object-valued entries reuse recursive flattening with paths relative to the child value. Scalar, null, or array values forced through `Entries` use a typed `/value` column. Direct member order is retained as base row order.

Direct member keys must be unique. Materialized inputs reject duplicates while opening. Incremental inputs track keys as entries are indexed and fail safely if a later duplicate is discovered; they never overwrite an earlier entry or expose a second row with ambiguous identity.

Alternative considered: rewrite the object to a temporary array of objects containing `name`. That copies data, loses a distinct identity for the synthetic key, risks colliding with real `name` fields, and prevents bounded large-map access.

### 4. Give the synthetic key a format-neutral non-pointer identity

Add a format-neutral object-member-key source identity, such as `ColumnSourceIdentity::ObjectKey`, and expose its durable saved-view key as `@key`, which cannot collide with canonical structured paths because their RFC 6901 representation begins with `/`. Keep the shared structured-path and object-key identities independent of any adapter even though JSON is their first implementation. The key column is text, is always source column zero, and defaults to display label `name`. If the initial child schema also claims `name`, the key label becomes `_key`. Existing late-label collision rules preserve established labels if `/name` arrives after initial rendering.

The direct object/map key is row data but not a value located at a child-relative structured path, so representing it as a fabricated `/name` pointer would be misleading and could collide with an actual property.

### 5. Add a lazy keyed-map entry store

For materialized inputs, the streaming selection visitor can emit owned member keys plus raw member values and feed the common entry projector. For large seekable inputs, add a keyed-object store parallel to the lazy JSON array store. It records, per logical row:

- the decoded member key;
- the raw value start/end boundary needed for independent seek and reparse;
- generation/fingerprint state already required by incremental stores.

A path-aware counting reader/Serde visitor locates the selected object and walks `MapAccess`. The detector reuses the first indexed entries rather than performing a separate full parse. Schema scanning observes projected child rows until the existing schema byte limit or end of the selected map. Navigation indexes additional entries, late fields produce schema deltas, and controlled full operations use the normal store contract.

Offset correctness must be proven across escaped keys, whitespace, nested values, commas consumed as lookahead, buffer boundaries, and surrounding metadata. The store reopens and seeks for row decoding, combines the recorded key with the reparsed value, and validates the source fingerprint before extending or reading indexed state.

Alternative considered: deserialize the selected object into `serde_json::Map` and then create an in-memory table. That is acceptable for small inputs but defeats the large-file architecture for multi-megabyte mappings and settings objects.

### 6. Surface requested and resolved interpretation

Retain both the requested mode and resolved shape in opened-table/source metadata. Table information reports values such as `object mode: auto → entries`, plus a short hint that `record` restores single-row behavior. Explicit modes report without an inference hint. Reload reuses the effective source option and performs fresh auto detection for the new source generation only when the effective mode remains `auto`.

Carry the requested and resolved modes from the adapter through opened-table metadata to saved-view serialization. When writing a saved view for a selected object/map, serialize the resolved mode as explicit `record` or `entries`, including when the source was opened with `auto`. Loading that saved view bypasses automatic detection unless an explicit CLI option takes precedence. Omit `object_mode` when the selected value is an array or scalar, or the source is a row stream, because no object interpretation was resolved. This makes saved views reproducible while allowing the unsaved default detector to improve over time and applies equally to future structured adapters.

Alternative considered: make inference silent. Silent structural reinterpretation would make false positives look like parser corruption and leave users without a discoverable remedy.

## Risks / Trade-offs

- **[A structurally regular record is falsely detected as keyed rows]** → Keep thresholds conservative, expose the resolved mode, and provide explicit `record` mode in CLI and saved views.
- **[A sparse or tiny keyed map is not automatically detected]** → Preserve correctness as one record and allow explicit `entries`; false negatives are safer than silent false positives.
- **[The first sampled entries are homogeneous but later entries differ]** → Lock the chosen shape, represent later scalar values through `/value`, and append later object fields through normal schema deltas.
- **[Streaming map offsets are sensitive to parser lookahead]** → Normalize and independently reparse recorded boundaries with fixtures spanning whitespace, escaped keys, nesting, and buffer boundaries.
- **[The synthetic key label collides with a real or late `name` field]** → Use distinct `@key` identity, `_key` for initial conflicts, and established late-label collision rules.
- **[A source contains duplicate direct member keys]** → Reject during materialized opening or upon incremental discovery without overwriting data or exposing ambiguous keyed rows.
- **[Automatic behavior changes existing output]** → Document the compatibility change and make `record` a stable opt-out that can be persisted per filename.
- **[Automated output changes when default detection improves]** → Recommend an explicit `record` or `entries` mode in reproducible scripts and saved views.
- **[Two active changes describe dependent JSON behavior]** → Integrate or archive `large-file-store` before implementing this change, then verify its deltas against the resulting main specs.

## Migration Plan

1. Integrate the `large-file-store` change so format-aware JSON stores and source options are the implementation baseline.
2. Add the object-mode option, CLI/saved-view schema, precedence, validation, and requested/resolved metadata without changing projection.
3. Add common detection and projection primitives plus materialized keyed-object support and identity/label handling.
4. Add the lazy keyed-map store and bounded detector reuse, then connect schema deltas, reload, and source information.
5. Add representative Elasticsearch fixtures and cross-format regression tests, then document the automatic behavior and `record`/`entries` overrides.

Rollback is configuration-safe: force `record` globally for affected invocations or in saved views while retaining the new parser code. A code rollback returns to the previous one-row selected-object behavior without changing source files.
