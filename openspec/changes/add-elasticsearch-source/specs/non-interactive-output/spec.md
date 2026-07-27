## ADDED Requirements

### Requirement: Non-interactive Elasticsearch target selection
Direct Elasticsearch output SHALL require either a valid native query or an explicitly selected index or data stream and SHALL never display or wait for the interactive target picker.

#### Scenario: Direct output with ES|QL
- **WHEN** direct output opens Elasticsearch with `source.query`
- **THEN** it executes the bounded query without performing interactive target selection

#### Scenario: Direct output with selected target
- **WHEN** direct output opens Elasticsearch with `source.table`
- **THEN** it validates the index or data stream and executes the generated bounded `FROM` query

#### Scenario: Direct output without query or target
- **WHEN** direct Elasticsearch output has neither `source.query` nor `source.table`
- **THEN** it writes no stdout, reports that `--query` or `--table` is required, and exits nonzero

#### Scenario: Interactive export may select target
- **WHEN** `--interactive --output table` opens Elasticsearch without a query or target
- **THEN** the startup picker resolves an index or data stream before table interaction and final export

### Requirement: Elasticsearch output query completion
Direct and post-interactive output SHALL await the latest required bounded Elasticsearch query and complete its active result before passing a source-neutral immutable projection to the selected output adapter.

#### Scenario: Direct output waits for ES|QL
- **WHEN** the initial ES|QL request is pending
- **THEN** the output driver writes no stdout until query execution and required bounded preparation succeed

#### Scenario: Interactive export waits for latest revision
- **WHEN** normal interactive quit requests final output while a newer Elasticsearch source revision is pending
- **THEN** final preparation awaits that latest revision before freezing and serializing the view

#### Scenario: Partial result policy
- **WHEN** Elasticsearch completes with an allowed partial result
- **THEN** stdout contains only the prepared result while partial-result warning metadata is reported on stderr

#### Scenario: Query preparation fails
- **WHEN** discovery, mappings, ES|QL, schema construction, or bounded traversal fails
- **THEN** no output-adapter bytes are emitted and the existing output failure contract applies

#### Scenario: Native query stays out of output
- **WHEN** ES|QL provenance exists during normal table serialization
- **THEN** stdout contains only the selected output adapter's bytes

### Requirement: Elasticsearch source-neutral conversion
Every output adapter SHALL consume Elasticsearch results through the shared table/view model rather than implementing Elasticsearch-specific serialization.

#### Scenario: ES|QL to text table
- **WHEN** an ES|QL result is rendered in table mode
- **THEN** its bounded typed rows, resolved columns, and local view configuration use the existing fixed-width output adapter

#### Scenario: Multivalued output cell
- **WHEN** an ES|QL result contains a structured multivalued cell
- **THEN** the selected output adapter renders it through the shared structured-cell representation without fetching the source again
