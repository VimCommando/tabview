## ADDED Requirements

### Requirement: Native SQLite query
The SQLite adapter SHALL accept `source.query` as one complete, single-statement, read-only, row-producing SQLite query; it SHALL prepare the query through the confined read-only session, expose its result as one implicit relation, and reject statements that mutate state, produce no table result, contain additional statements, or require side effects.

#### Scenario: Read-only select
- **WHEN** `source.query` is `SELECT id, name FROM users WHERE active = 1`
- **THEN** the prepared result columns and rows become the opened table

#### Scenario: Read-only common table expression
- **WHEN** a single row-producing `WITH ... SELECT ...` query is accepted by the confined SQLite engine
- **THEN** its result opens through the same native-query path

#### Scenario: Mutating statement
- **WHEN** `source.query` attempts `INSERT`, `UPDATE`, `DELETE`, schema mutation, attachment, or another state-changing operation
- **THEN** opening fails before any persistent database or sidecar state changes

#### Scenario: Multiple statements
- **WHEN** `source.query` contains more than one SQL statement
- **THEN** opening fails instead of executing or ignoring trailing statements

#### Scenario: Non-row-producing statement
- **WHEN** a prepared query has no tabular result metadata
- **THEN** opening fails with a clear read-only row-query diagnostic

#### Scenario: Native query bypasses table selection
- **WHEN** a valid native SQL query is supplied without `source.table`
- **THEN** its result is treated as the single selected implicit relation and no SQLite relation picker is displayed

### Requirement: Bounded native SQLite query
Tview SHALL compose its supported source filters, source sorting, and positive source limit over a valid native SQLite base query without changing the base query's internal semantics or escaping read-only confinement.

#### Scenario: Default native query limit
- **WHEN** a native SQLite query has no configured source limit
- **THEN** the outer Tview query retains at most 1,000 rows

#### Scenario: Source filter over query result
- **WHEN** a source filter references an unambiguous native-query result column
- **THEN** the adapter applies a bound outer predicate before the final Tview source limit

#### Scenario: Source sort over query result
- **WHEN** a source sort references an unambiguous native-query result column
- **THEN** the adapter applies safely quoted outer ordering before the final Tview source limit

#### Scenario: Query contains its own limit
- **WHEN** the native SQL base query contains `LIMIT`
- **THEN** its limit remains inside the derived result and Tview's hard source limit is still applied outside it

### Requirement: Native SQLite result schema and identity
Native SQLite query result metadata SHALL define the table columns directly, and stable row identity SHALL be unavailable unless the adapter can prove a unique durable identity from explicit result metadata.

#### Scenario: Query expression column
- **WHEN** a native query returns an expression or alias
- **THEN** its prepared result name and conservative declared-type metadata define the corresponding column

#### Scenario: Query changes columns
- **WHEN** a replacement native SQL query returns a different set or type of columns
- **THEN** SQLite publishes a new table definition and result atomically

#### Scenario: Identity cannot be proven
- **WHEN** an arbitrary native query does not expose a provably unique durable key
- **THEN** source-query replacement resets cursor-following and marks rather than guessing from row position

### Requirement: Native SQLite query provenance
The SQLite query artifact SHALL distinguish user-supplied base SQL from application-composed outer filtering, sorting, and limiting while exposing the final logical parameterized SQL and safe copyable representation.

#### Scenario: User SQL is active
- **WHEN** a native SQLite query opens successfully
- **THEN** query information preserves the user's base text and identifies the final composed SQL used for the bounded result

#### Scenario: Private extent probe
- **WHEN** execution uses an extra-row probe
- **THEN** the private probe does not replace the logical configured-limit artifact shown to the user

#### Scenario: Credentials absent
- **WHEN** query provenance is serialized or displayed
- **THEN** it contains no future remote SQLite credential material
