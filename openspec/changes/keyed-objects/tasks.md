## 1. Prerequisite and Source Options

- [ ] 1.1 Integrate or archive `large-file-store`, then verify this change's proposal and delta specs against the resulting main JSON, CLI, and saved-view requirements.
- [ ] 1.2 Add format-neutral `ObjectMode::{Auto, Record, Entries}` to shared open source options with `Auto` default and requested/resolved shape metadata.
- [ ] 1.3 Extend source-option merging and post-resolution adapter capability validation for CLI-over-saved precedence, JSON support, future structured-adapter reuse, explicit delimited/NDJSON conflicts, and selected array/scalar incompatibility without changing stdin buffering.

## 2. Shape Detection and Row Projection

- [ ] 2.1 Introduce a shared post-selection table-shape model that lets the JSON adapter resolve its JSON Pointer before choosing array rows, one object record, or object entries and can be reused by future structured adapters.
- [ ] 2.2 Implement the bounded 64-entry/1-MiB detector with minimum-entry, all-object, 75-percent shared-property, and consistent-value-kind rules.
- [ ] 2.3 Add detector unit tests for repository, pipeline, settings, alias, scalar-bearing record, two-entry section, heterogeneous-object, explicit-null-versus-absent evidence, byte-bound, and entry-bound cases.
- [ ] 2.4 Implement keyed-entry projection that preserves member order, flattens object children relative to each child, and maps forced scalar/null/array entries to typed `/value` cells.
- [ ] 2.5 Add projection tests for optional fields, nested objects, structured arrays, null/scalar maps, escaped keys, source order, and child-relative pointers.

## 3. Key Column and Schema Integration

- [ ] 3.1 Add a format-neutral object-key source identity with canonical saved-view key `@key`, text type metadata, and first-column ordering; represent structured child paths with adapter-neutral identities serialized using RFC 6901 JSON Pointer syntax.
- [ ] 3.2 Implement `name`/`_key` display-label selection and non-renaming late `/name` collision behavior.
- [ ] 3.3 Extend structured schema discovery and typed-row construction so keyed child fields share canonical path columns, missing fields differ from explicit nulls, types widen, and late paths append normally.
- [ ] 3.4 Extend structured saved-column resolution and column information to recognize and display the synthetic key identity independently of its label.

## 4. Materialized JSON Objects

- [ ] 4.1 Extend streaming JSON selection to retain direct member keys and raw values for selected object entries without fabricating an intermediate array, rejecting duplicate keys during materialized opening.
- [ ] 4.2 Apply `Auto`, `Record`, and `Entries` consistently to small/path-selected JSON objects while leaving selected arrays unchanged.
- [ ] 4.3 Add materialized adapter tests proving automatic and forced behavior, compatibility `Record` behavior, pointer-before-mode ordering, and scalar-entry forcing.

## 5. Incremental Keyed-Map Store

- [ ] 5.1 Implement path-aware streaming selected-object traversal that records unique decoded member keys and independently reparsable value boundaries across Serde lookahead, failing safely when a later duplicate is discovered.
- [ ] 5.2 Add a lazy keyed-object `TableStore` with partial/exact row counts, indexed row access, bounded forward indexing, schema deltas, scan/fold traversal, and controlled materialization.
- [ ] 5.3 Reuse initially indexed entries for automatic detection and schema discovery without a second full parse, honoring existing schema-scan and lazy thresholds.
- [ ] 5.4 Preserve generation fingerprints and failure-safe state when keyed-object files are replaced, truncated, malformed, or changed during incremental access.
- [ ] 5.5 Add offset/reparse tests covering whitespace, escaped and duplicate member keys, nested objects/arrays, commas, buffer boundaries, surrounding metadata, and nested JSON starting paths.
- [ ] 5.6 Add large-map tests proving bounded initial work, full first viewport rendering, progressive navigation, late columns, exact end detection, materialization, and no mixed-generation rows.

## 6. CLI, Saved Views, and Visibility

- [ ] 6.1 Add format-neutral `--object-mode auto|record|entries` parsing, help text, adapter capability validation, and CLI precedence tests.
- [ ] 6.2 Carry requested/resolved object-mode metadata from source opening into saved-view serialization; add `object_mode` parsing, semantic validation, schema definition, precedence, resolved-mode persistence, and non-object omission tests.
- [ ] 6.3 Surface requested and resolved object mode in table/source information, including the `record` override hint for automatic entries detection.
- [ ] 6.4 Preserve explicit CLI/saved-view modes across reload, re-run detection only for an effective `auto` mode, and prove saved explicit modes remain stable when detector defaults change.
- [ ] 6.5 Update README, CLI help, and saved-view examples with keyed-object behavior, detection criteria, compatibility implications, `@key`, and explicit `record`/`entries` hints for reproducible invocations.

## 7. End-to-End Verification

- [ ] 7.1 Add representative keyed-object fixtures modeled on repositories, pipelines, index settings, aliases, mappings, and nested node maps without depending on external diagnostic files.
- [ ] 7.2 Add end-to-end CLI/TUI tests for automatic table shape, first `name` column, child columns, source information, saved `@key` configuration, resolved-mode serialization, and `record` opt-out.
- [ ] 7.3 Add regressions proving array/scalar handling, object-record JSON, duplicate-key rejection, NDJSON cardinality, JSON Pointer selection, provisional schemas, and existing delimited inputs remain unchanged.
- [ ] 7.4 Run formatting, linting, default-feature tests, no-default-feature tests, and release build verification.
