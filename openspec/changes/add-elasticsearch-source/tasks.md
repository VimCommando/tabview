## 1. Source Target and CLI Foundations

- [ ] 1.1 Replace the path-only input representation with a `SourceTarget` that distinguishes local paths, stdin, and parsed URLs while preserving existing file and stdin behavior.
- [ ] 1.2 Add safe target display and persistence helpers that normalize `file://` and redact URL userinfo and secret-bearing components.
- [ ] 1.3 Implement format precedence across explicit CLI, saved format, unambiguous scheme registration, and existing local probing; register `libsql://` as SQLite and keep HTTP(S) ambiguous.
- [ ] 1.4 Add the feature-gated `InputFormat::Elasticsearch` value and validate compatible target kinds without probing remote HTTP content.
- [ ] 1.5 Generalize `--table` availability and validation and add feature-aware `--query`, including `--table`/`--query` conflict handling and rejection by non-native adapters.
- [ ] 1.6 Extend effective source options and CLI/saved/default merging with native query text while retaining explicit CLI precedence.
- [ ] 1.7 Add CLI and target-resolution tests covering local paths, file URIs, stdin, `libsql://`, ambiguous HTTP(S), feature-disabled formats, generic table selection, and native-query conflicts.

## 2. Native Query and Result Contracts

- [ ] 2.1 Introduce a source request model that carries optional native base text separately from filters, sort keys, and the hard source limit.
- [ ] 2.2 Replace SQLite-named query provenance with language-neutral SQL/ES|QL artifacts and typed value or identifier parameters.
- [ ] 2.3 Extend source-result metadata so complete/limited extent and remote partial status are independent and serializable to status/UI state.
- [ ] 2.4 Replace store-only source-query completion with an atomic result bundle containing `TableDefinition`, store, extent/partial metadata, and native query artifact.
- [ ] 2.5 Add schema compatibility detection, new-generation creation, durable column remapping, and stale configuration handling for schema-changing replacement.
- [ ] 2.6 Add Elasticsearch document stable identity using unique `_index`/`_id` tuples and preserve existing SQLite identities without position-based fallback.
- [ ] 2.7 Generalize the source-query coordinator to execute async network futures and blocking local jobs under one latest-revision contract.
- [ ] 2.8 Add unit tests for native query capability validation, language-neutral provenance, partial metadata, schema-changing atomic activation, stale revision discard, failure preservation, and identity reset/preservation.

## 3. Confined SQLite Native Queries

- [ ] 3.1 Investigate and document the pinned Turso APIs available to prove that one prepared statement is read-only, row-producing, and free of executable trailing SQL.
- [ ] 3.2 Extend the private SQLite facade with a narrow native row-query preparation path that retains storage read-only flags and verified `PRAGMA query_only`.
- [ ] 3.3 Reject mutation, attachment/detachment, state-changing pragmas, schema commands, multiple statements, and non-row-producing SQL before observable persistent side effects.
- [ ] 3.4 Build an implicit `TableDefinition` from prepared native-query result metadata with conservative type and identity behavior.
- [ ] 3.5 Compose source predicates, ordering, and the configured limit around native SQL as a derived table, rejecting ambiguous or unaddressable result columns.
- [ ] 3.6 Preserve configured base SQL separately from final composed SQL and private extent probes in the native query artifact.
- [ ] 3.7 Add accepted SELECT/CTE, duplicate-column, expression-column, schema replacement, unsupported composition, and query-provenance tests.
- [ ] 3.8 Add physical and logical before/after regression tests proving accepted and rejected native SQL does not modify database files, sidecars, schema, or rows.

## 4. Elasticsearch Client and Transport

- [ ] 4.1 Add a non-default `elasticsearch` Cargo feature with a pinned official client version and the minimal verified TLS feature set.
- [ ] 4.2 Add the Elasticsearch adapter module behind complete compile gates and verify the client dependency graph is absent without the feature.
- [ ] 4.3 Build client transport from the positional HTTP(S) endpoint and environment-backed API-key or username/password credentials with optional custom CA certificate.
- [ ] 4.4 Validate conflicting or incomplete credential modes and ensure transport construction, errors, debug output, and safe endpoint display redact secrets.
- [ ] 4.5 Add transport unit tests using mocked HTTP responses for endpoint parsing, authentication headers, custom CA validation errors, timeouts, and redaction.
- [ ] 4.6 Verify authentication and TLS configuration uses only the documented environment variables and exposes no connection-profile or credential CLI surface.

## 5. Elasticsearch Discovery and Mapping Catalog

- [ ] 5.1 Implement resolve-index discovery for open resources through the official client and deserialize indices, aliases, and data streams into adapter-owned metadata.
- [ ] 5.2 Filter dot-prefixed, hidden, closed, and unavailable resources; omit aliases and backing indices; sort indices and data streams deterministically.
- [ ] 5.3 Pass explicit `--table` values through safe ES|QL target rendering without discovery validation and route mapping, field-capability, or query rejection through normal Elasticsearch errors.
- [ ] 5.4 Fetch complete mappings for a selected index or data stream and flatten leaf, object, nested, multifield, and runtime metadata into a field catalog.
- [ ] 5.5 Fetch and merge field capabilities, retaining cross-index type conflicts and queryability metadata rather than choosing one mapping.
- [ ] 5.6 Add discovery and mapping fixtures covering ordinary indices, hidden/dot/system resources, closed indices, aliases omitted from the picker, explicit alias pass-through, data streams, backing indices, runtime fields, multifields, and conflicting mappings.

## 6. ES|QL Composition and Store

- [ ] 6.1 Generate a safe default ES|QL base query from a selected index or data stream, including `_index` and `_id` metadata and a hard default limit of 1,000.
- [ ] 6.2 Accept complete user ES|QL as an opaque base query without attempting to parse its `FROM` targets or fetching an inferred mapping catalog.
- [ ] 6.3 Compile supported source filters and sort keys into adapter-native ES|QL stages with safe identifier handling and bound values.
- [ ] 6.4 Apply the final configured hard limit and an `N + 1` extent probe when permitted without changing displayed logical provenance.
- [ ] 6.5 Execute structured JSON ES|QL requests asynchronously and deserialize columns, values, `is_partial`, warnings, and error responses.
- [ ] 6.6 Construct `TableDefinition` from ES|QL response columns, retain raw ES|QL types, and link mapping identities only when unambiguous.
- [ ] 6.7 Convert scalar values losslessly and preserve arrays, objects, oversized numeric values, and unsupported/source-specific types as structured or textual typed cells.
- [ ] 6.8 Implement the bounded in-memory Elasticsearch `TableStore`, result extent/partial metadata, query provenance, and optional document identity.
- [ ] 6.9 Abort or discard superseded requests, keep the prior successful result during pending work, and atomically activate schema-changing successful results.
- [ ] 6.10 Add ES|QL compiler and response fixtures covering generated and user queries, parameters, special identifiers, limits, partial responses, multivalues, mappings conflicts, schema transforms, aggregations, identity, timeout, and failure preservation.

## 7. Picker and Interactive Workflows

- [ ] 7.1 Generalize relation catalog entries so the startup picker can distinguish SQLite tables/views from Elasticsearch indices and data streams.
- [ ] 7.2 Add an Elasticsearch target picker with separate scrollable Indices and Data streams sections, deterministic selection, disabled/error states, cancellation, and small-terminal behavior.
- [ ] 7.3 Route interactive and interactive-output Elasticsearch startup without table/query through discovery and the picker before opening a result.
- [ ] 7.4 Extend Source Configuration with safe endpoint identity, selected target or base query, mapping/result fields, ES|QL capabilities, limit, extent/partial state, and pending/failure feedback.
- [ ] 7.5 Compose and apply Elasticsearch Source Configuration drafts through one asynchronous result-bundle replacement.
- [ ] 7.6 Generalize the query popup and clipboard action to label, display, and copy SQL or ES|QL artifacts and parameters without secrets.
- [ ] 7.7 Update status/footer information for Elasticsearch visible/source counts, limited/unknown extent, partial results, and query progress.
- [ ] 7.8 Add interaction and render tests for discovery, picker sections/filtering, explicit selection, query-only startup, Source Configuration, schema-changing replacement, query display, cancellation, errors, and narrow terminals.

## 8. Saved Views and Reload

- [ ] 8.1 Add `source.query` to saved-view parsing, validation, merge precedence, and schema documentation with mutual exclusion from `source.table`.
- [ ] 8.2 Extend saved-view target matching and generated filenames/identities for sanitized remote URL targets without persisting credentials.
- [ ] 8.3 Serialize configured native query text separately from structured source operations and exclude derived SQL/ES|QL artifacts and private probes.
- [ ] 8.4 Persist Elasticsearch format, selected index/data stream or ES|QL query, limit, source operations, and compatible column configuration through the nested schema.
- [ ] 8.5 Reopen and reload Elasticsearch targets through a new generation, re-run discovery/mappings when applicable, and atomically remap compatible saved/view state.
- [ ] 8.6 Add saved-view round-trip, override, conflict, sanitization, generated YAML, schema-changing reload, and feature-disabled compatibility tests for SQL and ES|QL sources.

## 9. Non-Interactive Output

- [ ] 9.1 Require explicit table or native query for direct Elasticsearch output and fail before stdout when target selection would otherwise require a picker.
- [ ] 9.2 Allow interactive-output startup to use the Elasticsearch picker and carry the selected target through final serialization.
- [ ] 9.3 Await the latest Elasticsearch revision and complete its bounded active result before preparing an immutable source-neutral output projection.
- [ ] 9.4 Report allowed partial-result warnings on stderr while reserving stdout for adapter bytes; preserve the no-partial-output-on-failure contract.
- [ ] 9.5 Add direct and post-interactive output tests for selected targets, native queries, missing selection, pending replacement, partial results, failures, multivalued cells, and clean stdout/stderr separation.

## 10. Integration Verification and Documentation

- [ ] 10.1 Add versioned Docker-backed Elasticsearch integration fixtures with visible/hidden indices, a data stream, conflicting mappings, multivalued fields, and representative ES|QL transforms.
- [ ] 10.2 Add live integration tests for resolve-index discovery, mappings, field capabilities, generated ES|QL, user ES|QL, authentication, limits, partial/error handling, and read-only behavior.
- [ ] 10.3 Verify existing delimited, JSON, NDJSON, SQLite table, saved-view, interactive, and output tests remain unchanged for local invocations.
- [ ] 10.4 Record Elasticsearch client version compatibility, feature/dependency/build-size impact, TLS backend choice, supported environment variables, and minimum privileges.
- [ ] 10.5 Update README usage, format/target resolution, generic `--table` and `--query`, Elasticsearch picker, mappings versus result schema, limits, partial results, authentication, and saved-view examples.
- [ ] 10.6 Run formatting, default-feature and no-default-feature builds, Elasticsearch-feature builds, Clippy for every supported feature combination, unit/integration tests, and supported platform checks.
