## Why

Tview can inspect local files and SQLite databases through a shared typed table model, but it cannot connect directly to Elasticsearch or use a source-native query supplied uniformly from the CLI. Adding a URL-aware source target and a generic native-query boundary lets Elasticsearch use ES|QL without proliferating source-specific arguments and prepares the same architecture for future remote database adapters.

## What Changes

- Add an optional `elasticsearch` Cargo feature backed by the official Elasticsearch Rust client.
- Generalize the positional input from a local path/stdin value into a source target that can represent local paths, `file://` URIs, stdin, and remote URLs.
- Resolve format by explicit `--format` first, then by an unambiguous URL scheme such as `libsql://`, then by existing local signature, extension, and bounded content probing. Ambiguous `http://` and `https://` targets require an explicit format.
- Add `elasticsearch` to `--format` and interpret an HTTP(S) target selected with that format as an Elasticsearch endpoint.
- Generalize `--table <name>` as a relation/target selector and add a generic `--query <string>` containing the native query language selected by the resolved source format. `--table` and `--query` are mutually exclusive.
- For Elasticsearch, treat `--query` as complete ES|QL. Without a query, use `--table` or an interactive picker to choose a visible open index or data stream and generate a bounded `FROM` query.
- Discover Elasticsearch picker entries through the structured resolve-index API, show non-hidden open indices and data streams whose names do not begin with `.`, and omit aliases from the picker. Pass an explicit `--table` target through without discovery validation so aliases and other valid ES|QL targets can still be entered directly.
- Read the selected Elasticsearch target's mappings and field capabilities for field discovery, type/conflict metadata, and source-operation composition; treat the ES|QL response columns as authoritative for the rendered result schema.
- Execute bounded ES|QL asynchronously, preserve typed and multivalued response values, distinguish partial results from limit truncation, and retain the last valid result while a replacement is pending or fails.
- Configure Elasticsearch authentication and custom CA trust only through environment variables in this change; do not add credential CLI arguments or connection profiles.
- Generalize native-query provenance, source-query replacement, saved-view source state, and source-operation composition so native SQL and ES|QL queries may change result schemas safely.
- Permit read-only, row-producing native SQLite queries through the same `--query` surface while retaining storage-level read-only and query-only confinement.
- Keep remote Turso/libSQL execution outside this change; recognizing `libsql://` as SQLite may produce an explicit unsupported-remote diagnostic until that adapter is introduced.

## Capabilities

### New Capabilities

- `elasticsearch-data-source`: Elasticsearch connection behavior, target discovery, mapping-aware schema discovery, ES|QL execution, bounded results, typed values, identity, partial-result handling, and failure behavior.

### Modified Capabilities

- `data-ingestion`: Represent and resolve remote URL targets in addition to local files, file URIs, and stdin.
- `cli-compatibility`: Add Elasticsearch format selection, unambiguous URL-scheme inference, generic `--table`, and generic native `--query`.
- `table-source-model`: Support language-neutral native-query artifacts and atomic source replacements whose result schema may change.
- `table-operations`: Compose source filters, sorts, and limits around a source-native base query without changing local view semantics.
- `sqlite-data-source`: Accept confined read-only native SQL queries and derive their result metadata through the existing SQLite adapter.
- `saved-views`: Persist native source queries and remote source targets without persisting credentials.
- `non-interactive-output`: Define Elasticsearch target selection, query completion, and failure behavior for direct and post-interactive output.

## Impact

- Affects source-target parsing, format resolution, CLI/config merging, saved-view matching and serialization, relation discovery, table-definition replacement, query provenance, asynchronous source work, picker UI, and output preparation.
- Adds the optional official `elasticsearch` client and its HTTP/TLS dependency graph; the feature-disabled build omits Elasticsearch format parsing, dispatch, and discovery.
- Extends the current SQLite-focused relation and query contracts into source-neutral native-query contracts while preserving existing file, JSON, NDJSON, and SQLite behavior.
- Introduces network, authentication, TLS, cluster-version, mapping-conflict, partial-result, and remote-timeout test surfaces.
