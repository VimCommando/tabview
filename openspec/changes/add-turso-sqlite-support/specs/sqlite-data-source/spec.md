## ADDED Requirements

### Requirement: SQLite source resolution
The system SHALL open local SQLite-format path inputs through Turso when SQLite is selected explicitly or detected from the strong database signature before text decoding.

#### Scenario: SQLite file with arbitrary extension
- **WHEN** a seekable path begins with the `SQLite format 3\0` signature in automatic format mode
- **THEN** the system opens it through the SQLite adapter regardless of extension

#### Scenario: Explicit SQLite format
- **WHEN** a user explicitly selects SQLite for a local path
- **THEN** the SQLite adapter is used and reports an actionable database error for an unsupported file

#### Scenario: Non-SQLite automatic input
- **WHEN** a seekable input lacks the SQLite signature and SQLite was not selected
- **THEN** existing format resolution continues normally

### Requirement: SQLite relation discovery and selection
The system SHALL classify SQLite schema objects, expose user-facing ordinary tables and compatible ordinary views as selectable candidates, retain actionable diagnostics for incompatible ordinary views and explicitly requested virtual tables, exclude shadow and internal objects, and open exactly one selected relation. An ordinary view SHALL be selectable only after a non-mutating queryability and result-metadata probe succeeds through the pinned Turso version. Virtual tables SHALL NOT be selectable in this change regardless of module availability.

#### Scenario: Explicit table selection
- **WHEN** `--table` or saved `source.table` resolves an ordinary table or compatible ordinary view
- **THEN** the SQLite adapter opens it without displaying the table-selection modal

#### Scenario: One table is selected automatically
- **WHEN** discovery finds exactly one selectable candidate and no table is requested
- **THEN** the adapter opens it automatically without displaying the modal

#### Scenario: Multiple relations open an interactive modal
- **WHEN** discovery finds multiple selectable candidates, no table is requested, and execution is interactive or post-interactive output
- **THEN** the application displays a “Select table” modal before opening a row stream

#### Scenario: Multiple relations in direct batch output
- **WHEN** discovery finds multiple selectable candidates, no table is requested, and execution is direct batch output
- **THEN** opening fails without waiting for input or writing stdout and the diagnostic requires `--table` or saved `source.table`

#### Scenario: Modal selection is confirmed
- **WHEN** the user confirms a table or view in the modal
- **THEN** the adapter opens only that relation

#### Scenario: Modal selection is cancelled
- **WHEN** the user cancels the modal
- **THEN** opening ends cleanly without creating a relation row stream

#### Scenario: Requested table is missing
- **WHEN** the requested name is absent from the classified schema catalog
- **THEN** opening fails before constructing a query from the unresolved name

#### Scenario: Compatible ordinary view
- **WHEN** an ordinary view successfully prepares an application-generated zero-row query and supplies result metadata
- **THEN** the view is a selectable candidate with conservative column metadata and no stable row identity

#### Scenario: Incompatible ordinary view in the picker
- **WHEN** the table picker is required and a discovered ordinary view fails its compatibility probe
- **THEN** the modal displays that view disabled with its Turso compatibility reason

#### Scenario: Incompatible ordinary view is explicitly requested
- **WHEN** `--table` or saved `source.table` names an ordinary view that fails its compatibility probe
- **THEN** opening fails with the recorded compatibility reason rather than reporting that the view is missing

#### Scenario: Virtual table is requested
- **WHEN** `--table` or saved `source.table` names a discovered virtual table
- **THEN** opening fails with a clear unsupported-virtual-table diagnostic even if Turso registers that module

#### Scenario: Shadow and internal objects
- **WHEN** discovery encounters a virtual-table shadow object or SQLite internal object
- **THEN** the object is omitted from the user-facing catalog and cannot be selected

#### Scenario: No selectable relations
- **WHEN** discovery finds no selectable ordinary table or compatible ordinary view
- **THEN** opening fails clearly and distinguishes an empty database from one containing only unsupported or incompatible objects

#### Scenario: Disabled view does not force selection
- **WHEN** discovery finds exactly one selectable candidate plus one or more incompatible ordinary views
- **THEN** the selectable candidate is opened automatically without displaying the modal

#### Scenario: Empty database
- **WHEN** discovery finds no user table, ordinary view, virtual table, or unavailable user object
- **THEN** opening fails with a clear empty-database message

#### Scenario: Identifier requires quoting
- **WHEN** the selected relation name contains spaces, quotes, or SQL-significant characters
- **THEN** the adapter resolves it from metadata and safely quotes the generated identifier

### Requirement: Bounded SQLite source query
Every SQLite table view SHALL be produced from an application-generated source query containing the selected relation, source filters, source sorting, and a positive result limit applied in that order.

#### Scenario: Default source query
- **WHEN** no source filters, source sort, or saved source limit is configured
- **THEN** the adapter executes the equivalent of `SELECT * FROM <selected-table> LIMIT 1000` with safe identifier handling

#### Scenario: Source filtering and sorting
- **WHEN** source filters and source sort keys are active
- **THEN** generated `WHERE` and `ORDER BY` clauses execute before the source limit

#### Scenario: Configured source limit
- **WHEN** saved `source.limit` is a positive integer
- **THEN** every source-query replacement uses that limit instead of the SQLite default

#### Scenario: View filtering reduces visible rows
- **WHEN** a view filter hides rows from a limited SQLite source result
- **THEN** the adapter does not fetch additional database rows to refill the visible result

### Requirement: SQLite-native source operations
The SQLite adapter SHALL compile supported typed source filters and source sort keys directly into parameterized SQLite `WHERE` and `ORDER BY` behavior without attempting to reproduce Tabview view semantics.

#### Scenario: Supported source predicate
- **WHEN** a source filter uses a supported equality, inequality, ordered comparison, contains, prefix, or null-test operation
- **THEN** the adapter generates a parameterized SQLite predicate using the resolved source column

#### Scenario: Source-native ordering
- **WHEN** a source sort is applied
- **THEN** SQLite ordering determines which rows enter the limited working set

#### Scenario: Unsupported source operation
- **WHEN** a requested source operation cannot be represented by the SQLite source-query vocabulary
- **THEN** the operation is rejected clearly without materializing the unbounded database

#### Scenario: Complex local operation
- **WHEN** the user applies regex, rendered-value, natural, semantic-version, IP, date, boolean, or custom numeric behavior as a view operation
- **THEN** Tabview evaluates it only over the bounded SQLite source result

### Requirement: SQLite result extent
The SQLite result SHALL distinguish a complete source result from one truncated by the configured source limit and SHALL report visible rows separately.

#### Scenario: Query has fewer rows than its limit
- **WHEN** the source query reaches end of stream before the configured limit
- **THEN** result extent is complete with the fetched source-row count

#### Scenario: Query exceeds its limit
- **WHEN** the source query has more matching rows than the configured limit
- **THEN** Tabview retains at most the limit and reports the source result as truncated

#### Scenario: Local filter hides rows
- **WHEN** a view filter leaves 17 visible rows from 1,000 fetched source rows
- **THEN** status distinguishes 17 visible rows from the 1,000-row limited source result

### Requirement: SQLite source-query provenance
Every successfully compiled SQLite source result SHALL retain its logical parameterized SQL, bound values, and a safely rendered copyable SQL statement using the configured limit, independently of any private extra-row probe used for extent detection.

#### Scenario: Display current SQL
- **WHEN** the user requests the current source query
- **THEN** the TUI displays a copyable statement representing the selected table, source filters, source sorting, and limit

#### Scenario: View transforms are active
- **WHEN** view filters or view sorts are active
- **THEN** the SQL display identifies them as local transformations not represented by the source SQL

#### Scenario: View-only change
- **WHEN** only the view filter or view sort changes
- **THEN** the current source SQL artifact remains unchanged

### Requirement: Asynchronous SQLite query replacement
The system SHALL execute SQLite source-query changes asynchronously, retain the last valid result while a replacement is pending, and activate only the latest successful revision.

#### Scenario: Source query is slow
- **WHEN** a filter or sort requires a long database scan
- **THEN** the TUI remains responsive and reports query progress while retaining the prior result

#### Scenario: Query is superseded
- **WHEN** a newer source query is requested before an older query completes
- **THEN** stale work is cancelled or discarded and cannot replace the newer result

#### Scenario: Replacement fails
- **WHEN** compilation, execution, or initial fetch fails
- **THEN** the error is reported and the prior source result and view state remain active

#### Scenario: Direct batch initial query
- **WHEN** direct batch output opens a SQLite table
- **THEN** output preparation awaits the initial bounded source-query revision before writing stdout

#### Scenario: Post-interactive pending query
- **WHEN** a user normally quits an interactive transformation while the latest requested source query is pending
- **THEN** final output preparation awaits that revision and emits no partial output if it fails

### Requirement: SQLite row identity
SQLite result rows SHALL use a stable database identity when available and SHALL define identity-dependent behavior when no stable key exists.

#### Scenario: Rowid table
- **WHEN** the selected table has an accessible SQLite rowid
- **THEN** rowid is retained as hidden identity metadata and may be used as a stable ordering tie-breaker

#### Scenario: Without-rowid table
- **WHEN** the selected table declares a primary key and has no rowid
- **THEN** the primary-key tuple is retained as stable row identity

#### Scenario: Relation has no stable key
- **WHEN** an ordinary view is selected or another relation cannot provide stable row identity across source-query replacements
- **THEN** cursor and mark state are reset rather than mapped by result position

### Requirement: Tabview-enforced read-only behavior
The SQLite adapter SHALL keep the raw Turso connection private, expose only typed discovery, schema, source-query, and row-fetch operations, enable and verify `PRAGMA query_only=ON`, and provide no production path for arbitrary or mutating SQL.

#### Scenario: User actions preserve logical database contents
- **WHEN** a user opens, selects, filters, sorts, navigates, displays SQL, reloads, and closes a SQLite relation
- **THEN** the database's persistent schema and table contents remain unchanged

#### Scenario: Production code requests database access
- **WHEN** UI or table-store code accesses SQLite
- **THEN** it uses the read-only facade rather than a raw connection or general execute method

#### Scenario: Confinement cannot be verified
- **WHEN** `PRAGMA query_only=ON` cannot be enabled and verified
- **THEN** opening fails before schema or row queries are exposed

#### Scenario: Mutation is attempted in a regression test
- **WHEN** a mutation is submitted through the configured low-level test connection
- **THEN** Turso rejects it as read-only

#### Scenario: Turso performs file bookkeeping
- **WHEN** Turso changes file metadata or engine-managed journal, WAL, or shared-memory state while reading
- **THEN** the guarantee remains scoped to persistent logical schema and data rather than byte-for-byte file identity

### Requirement: Incremental limited SQLite store
The system SHALL fetch and cache the bounded SQLite source result incrementally through `TableStore`.

#### Scenario: Initial viewport is bounded
- **WHEN** a limited SQLite source result is opened
- **THEN** only rows required for initial rendering and truncation bookkeeping are fetched before first display

#### Scenario: Forward navigation extends the cache
- **WHEN** navigation reaches beyond cached rows but remains inside the source limit
- **THEN** the store advances the active result stream in bounded chunks

#### Scenario: Backward navigation uses cached rows
- **WHEN** a previously fetched row is requested
- **THEN** the store returns it without a per-row database query

#### Scenario: Local materialization is required
- **WHEN** a view operation requires complete local evaluation
- **THEN** materialization drains at most the fixed limited source result

#### Scenario: Output adapter requires complete rows
- **WHEN** a source-neutral output adapter requests complete rows and stable widths
- **THEN** the store drains at most the fixed limited source result before serialization

### Requirement: Typed SQLite values
The SQLite adapter SHALL map Turso null, integer, real, text, and blob values directly into typed table-model values without first converting them to display text.

#### Scenario: Null differs from empty text
- **WHEN** a row contains both `NULL` and empty text
- **THEN** the model preserves distinct raw values

#### Scenario: Numeric kinds remain numeric
- **WHEN** a row contains integer and real values
- **THEN** the model preserves their numeric kinds

#### Scenario: Blob remains binary
- **WHEN** a row contains a blob
- **THEN** the model preserves its bytes without interpreting them as source text

### Requirement: SQLite declared types are hints
The SQLite adapter SHALL preserve a column's raw declared type for inspection and SHALL use SQLite affinity only as an initial logical-type hint. INTEGER, REAL, and TEXT affinity SHALL initially hint `Integer`, `Float`, and `Text`; an explicit `BLOB` declaration SHALL initially hint `Binary`; NUMERIC affinity, no declaration, and `ANY` SHALL initially hint `Unknown`. Runtime Turso values SHALL remain authoritative and SHALL widen the observed column profile when they contradict the hint.

#### Scenario: Conventional affinity hint
- **WHEN** an ordinary table column has INTEGER, REAL, TEXT, or explicit BLOB affinity
- **THEN** its initial logical type is hinted as `Integer`, `Float`, `Text`, or `Binary` respectively and its raw declaration remains inspectable

#### Scenario: Semantic-looking numeric declaration
- **WHEN** a column is declared `BOOLEAN`, `DATE`, `DATETIME`, `DECIMAL`, or another spelling with NUMERIC affinity
- **THEN** its initial logical type is `Unknown` and Tabview does not infer Boolean, temporal, or exact-decimal semantics from the spelling

#### Scenario: Untyped or ANY column
- **WHEN** a column has no declared type or is declared `ANY`
- **THEN** its initial logical type is `Unknown`

#### Scenario: Runtime value contradicts declaration
- **WHEN** a non-STRICT column's returned storage classes do not agree with its declared hint
- **THEN** each cell retains its actual typed value and the column profile widens normally, including to `Mixed` when required

#### Scenario: Null under a declared hint
- **WHEN** a hinted column returns `NULL`
- **THEN** the null remains a typed null and does not by itself invalidate the hint

#### Scenario: Strict table declaration
- **WHEN** a column belongs to a STRICT table
- **THEN** Tabview uses the same hint and runtime-value rules rather than adding a separate cell conversion path

#### Scenario: View expression lacks declaration
- **WHEN** prepared metadata for a view expression provides no reliable declared type
- **THEN** the column begins `Unknown` and its profile is inferred from returned values

### Requirement: SQLite reload and failure behavior
The system SHALL reopen the selected relation into a new source generation on reload and SHALL report Turso or compatibility failures without corrupting the last valid view.

#### Scenario: Reload selected relation
- **WHEN** a user reloads SQLite input
- **THEN** the system reopens the same relation and reapplies saved source and view configuration through stable identities

#### Scenario: Unsupported construct
- **WHEN** Turso cannot open or query a selected relation
- **THEN** the error is reported clearly, no database mutation is issued, and any prior valid view remains unchanged
