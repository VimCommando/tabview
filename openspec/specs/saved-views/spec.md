## Purpose

Define user-defined saved view configuration files, matching, validation, application, serialization, and writing behavior.

## Requirements

### Requirement: Saved view discovery
When compiled with the `saved-views` feature, the system SHALL discover user-defined saved view files from `$XDG_CONFIG_HOME/tabview/views`, or `~/.config/tabview/views` when `XDG_CONFIG_HOME` is unset, including files ending in `.yml` or `.yaml`.

#### Scenario: Discover views from config directory
- **WHEN** a user opens a file and saved views exist under `~/.config/tabview/views`
- **THEN** the system loads candidate `.yml` and `.yaml` view files before initializing the table view

#### Scenario: Missing view directory
- **WHEN** the saved view directory does not exist
- **THEN** the system opens the input with existing default behavior

#### Scenario: Duplicate yml and yaml stems
- **WHEN** both `cat-shards.yml` and `cat-shards.yaml` exist in the saved view directory
- **THEN** the system loads `cat-shards.yml`, ignores `cat-shards.yaml`, logs the conflict, and records a TUI warning

#### Scenario: Saved views feature disabled
- **WHEN** the binary is compiled without the `saved-views` feature
- **THEN** the system does not discover or apply saved views

### Requirement: Saved view schema
The system SHALL ship and document a schema file that validates the supported saved view YAML structure, including `name`, top-level `locale`, `filenames`, `columns`, column visibility, column type aliases, format values, width values, alignment values, numeric masks, sort state, and filter state.

#### Scenario: Editor validation
- **WHEN** a user configures a YAML language server with the shipped schema
- **THEN** valid saved view files using the supported structure validate without schema errors

#### Scenario: Invalid enum value
- **WHEN** a saved view sets a column `type` or `format` to an unsupported value
- **THEN** schema validation reports the field as invalid

### Requirement: Saved view validation
The system SHALL validate saved view files structurally and semantically before applying them.

#### Scenario: Invalid YAML file
- **WHEN** a saved view file contains invalid YAML
- **THEN** the system ignores that view file, records a non-fatal warning, and continues opening the input

#### Scenario: Invalid regex pattern
- **WHEN** a saved view filename pattern is classified as a regex but does not compile
- **THEN** the system ignores that pattern, records a non-fatal warning, and continues evaluating other patterns and views

#### Scenario: Invalid numeric mask
- **WHEN** a number column uses `format: mask` with a mask outside the supported mask grammar
- **THEN** the system ignores the mask for that column, records a non-fatal warning, and falls back to plain display for that column

#### Scenario: Invalid POSIX locale
- **WHEN** a saved view sets an unsupported top-level POSIX-style `locale`
- **THEN** the system logs the invalid locale, records a TUI warning, and falls back to `en_US`

#### Scenario: One view per file
- **WHEN** a saved view file is loaded
- **THEN** the system treats the file as exactly one saved view whose canonical name is the file stem

### Requirement: Filename matching
The system SHALL match saved views against the opened input basename using exact, glob, and regex filename patterns while following platform filename case behavior.

#### Scenario: Exact filename match
- **WHEN** a saved view includes `cat_shards.txt` in `filenames` and the opened input basename is `cat_shards.txt`
- **THEN** the saved view matches the input as an exact match

#### Scenario: Glob filename match
- **WHEN** a saved view includes `*shards*` in `filenames` and the opened input basename is `cat_shards.txt`
- **THEN** the saved view matches the input as a glob match

#### Scenario: Regex filename match
- **WHEN** a saved view includes `^cat_.*txt$` in `filenames` and the opened input basename is `cat_shards.txt`
- **THEN** the saved view matches the input as a regex match

#### Scenario: Multiple matching views
- **WHEN** more than one saved view matches the opened input
- **THEN** the system chooses a deterministic view using exact matches before glob matches before regex matches, then lexicographic view file path order within the same match rank

#### Scenario: Parent directory ignored
- **WHEN** a saved view filename pattern matches a parent directory name but not the opened input basename
- **THEN** the saved view does not match the input

#### Scenario: Platform case behavior
- **WHEN** a saved view filename pattern differs from the opened input basename only by letter case
- **THEN** the system matches or rejects it according to the platform filename case behavior

### Requirement: Saved view selection overrides
When compiled with the `saved-views` feature, the system SHALL apply matching saved views automatically by default and SHALL provide CLI overrides to force a saved view by canonical name or disable saved views for the invocation.

#### Scenario: Automatic view selection
- **WHEN** a user opens an input whose basename matches a valid saved view and no saved view override flag is present
- **THEN** the system applies the matching saved view automatically

#### Scenario: Force saved view by name
- **WHEN** a user runs `tabview --view cat-shards cat_nodes.txt` and `cat-shards.yml` exists
- **THEN** the system applies that saved view even if the input basename does not match the view's `filenames`

#### Scenario: Force saved view with extension
- **WHEN** a user runs `tabview --view cat-shards.yaml cat_nodes.txt` and `cat-shards.yml` exists
- **THEN** the system normalizes away the `.yaml` extension and applies the `cat-shards` saved view

#### Scenario: Disable saved views
- **WHEN** a user runs `tabview --no-view cat_shards.txt`
- **THEN** the system opens the input without discovering or applying saved views

#### Scenario: Missing forced view
- **WHEN** a user runs `tabview --view missing data.txt` and no saved view has that name
- **THEN** the system reports a clear CLI error and does not start the viewer

### Requirement: Column matching
The system SHALL apply column configuration sparsely by matching configured column keys to loaded table headers case-insensitively.

#### Scenario: Exact column key wins
- **WHEN** `columns` contains both `count` and `*count` and the table has a `Count` header
- **THEN** the system applies the exact `count` configuration to that column

#### Scenario: Wildcard column key
- **WHEN** `columns` contains `*count` and the table has `docs_count` and `store_count` headers
- **THEN** the system applies the wildcard configuration to both matching columns unless an exact configuration also exists

#### Scenario: Missing configured column
- **WHEN** a saved view configures a column key that matches no loaded header
- **THEN** the system ignores that column configuration and records a non-fatal warning

### Requirement: Column type metadata
The system SHALL support string, number, and boolean column type families with subtype aliases for text, date, float, integer, semantic version, IP address, character boolean, bit boolean, and word boolean.

#### Scenario: Broad type aliases
- **WHEN** a column sets `type: string`, `type: number`, or `type: boolean`
- **THEN** the system maps the value to the default subtype for that type family

#### Scenario: Subtype aliases
- **WHEN** a column sets `type: text`, `type: date`, `type: integer`, `type: semver`, `type: ip`, `type: char`, `type: bit`, or `type: word`
- **THEN** the system maps the value to the corresponding typed column subtype

#### Scenario: Type-aware sort
- **WHEN** a saved view gives a column an explicit type and the user sorts that column
- **THEN** the system uses the saved type metadata to select the appropriate comparison semantics when that subtype is implemented

#### Scenario: ISO 8601 date type
- **WHEN** a column sets `type: date`
- **THEN** the system parses ISO 8601 date/time values for chronological sorting where values parse successfully

#### Scenario: IP address type
- **WHEN** a column sets `type: ip`
- **THEN** the system treats the column as a string-family IP subtype and supports IPv4 and IPv6 parsing for IP-aware operations

#### Scenario: Loose semantic version type
- **WHEN** a column sets `type: semver`
- **THEN** the system parses values accepted by the selected SemVer parser, including loose version forms the parser supports

#### Scenario: Boolean subtype values
- **WHEN** a column sets `type: word`, `type: bit`, or `type: char`
- **THEN** the system recognizes `true`/`false` and `yes`/`no` for word booleans, `1`/`0` for bit booleans, and `y`/`n` for character booleans

### Requirement: Display formatting
The system SHALL apply saved display formatting to rendered cell values without changing raw cell values.

#### Scenario: Plain format
- **WHEN** a column uses `format: plain`
- **THEN** the system renders cell values without display transformation

#### Scenario: Locale number format
- **WHEN** a number column uses `format: locale` and the saved view does not set `locale`
- **THEN** the system renders numeric values with grouping and decimal separators using the POSIX-style system locale, falling back to `en_US` if system locale detection or lookup fails

#### Scenario: Top-level locale override
- **WHEN** a saved view sets top-level `locale: en_US` and a number column uses `format: locale`
- **THEN** the system renders locale-formatted values using the saved view locale instead of the system locale

#### Scenario: Numeric mask format
- **WHEN** a number column uses `format: mask` and `mask: "0.00"`
- **THEN** the system renders numeric values with two decimal places

#### Scenario: Numeric mask overrides locale
- **WHEN** a saved view sets top-level `locale: de_DE` and a number column uses `format: mask` with `mask: "#,##0.00"`
- **THEN** the system renders the value according to the mask grammar rather than substituting locale-specific separators

#### Scenario: String case format
- **WHEN** a string column uses `format: uppercase` or `format: lowercase`
- **THEN** the system renders that column's cell values using the requested case transformation

#### Scenario: Raw and rendered matching
- **WHEN** saved view formatting changes the rendered value for a cell
- **THEN** search and text or regex filters can match either the raw cell value or the rendered cell value

### Requirement: Column width and alignment metadata
The system SHALL use saved column width and alignment metadata to initialize the table layout while preserving existing interactive layout controls.

#### Scenario: Numeric width
- **WHEN** a column sets `width: 20`
- **THEN** the system initializes that column width to 20 display characters subject to existing terminal constraints

#### Scenario: Header width
- **WHEN** a column sets `width: header`
- **THEN** the system initializes that column width from the display width of the header

#### Scenario: Content width
- **WHEN** a column sets `width: content`
- **THEN** the system initializes that column width from the widest materialized content value in that column

#### Scenario: Alignment override
- **WHEN** a number column sets `align: left`
- **THEN** the system left-aligns rendered data cells for that column instead of using the numeric default

#### Scenario: Interactive width changes still work
- **WHEN** a saved view initializes column widths and the user presses existing width adjustment keys
- **THEN** the system adjusts widths using the existing interactive behavior

### Requirement: Column visibility metadata
The system SHALL use saved column visibility metadata to initialize which columns are shown in the table viewport.

#### Scenario: Visible omitted defaults to true
- **WHEN** a saved view configures a column without `visible`
- **THEN** the system treats that column as visible

#### Scenario: Hidden column from saved view
- **WHEN** a saved view configures a column with `visible: false`
- **THEN** the system keeps the column in the table model but excludes it from viewport rendering and horizontal navigation

#### Scenario: Hidden column remains available to data operations
- **WHEN** a saved view hides a column
- **THEN** the system preserves that column's raw values for reload, sorting metadata, active filters, and future show-column commands

### Requirement: Saved view serialization
The system SHALL serialize the current runtime view configuration to saved view YAML that conforms to the shipped schema.

#### Scenario: Serialize loaded view
- **WHEN** a saved view was loaded from disk and the user opens the view modal
- **THEN** the displayed YAML reflects the current runtime view configuration and identifies the loaded saved view filename

#### Scenario: Serialize new view placeholder
- **WHEN** no saved view was loaded from disk and the user opens the view modal for `foo.bar.csv`
- **THEN** the displayed target filename is `foo.bar.yml` under the saved views directory

#### Scenario: Serialize interactive column changes
- **WHEN** the user changes column widths or hides and shows columns before opening the view modal
- **THEN** the displayed YAML includes the current widths and `visible` values for affected columns

#### Scenario: Serialize only changed column state
- **WHEN** a column has no saved metadata and no interactive view-state changes
- **THEN** the displayed YAML omits that column from `columns`

#### Scenario: Serialize current filename only
- **WHEN** a saved view loaded with multiple filename patterns is displayed in the view modal
- **THEN** the generated YAML includes only the current input filename in `filenames`

#### Scenario: Serialize default locale omission
- **WHEN** locale formatting is using auto-detected or default locale behavior
- **THEN** the generated YAML omits top-level `locale`

#### Scenario: Serialize placeholder name
- **WHEN** no saved view was loaded for `cat_shards.txt`
- **THEN** the generated YAML includes `name: cat_shards`

#### Scenario: Serialize sort and filters
- **WHEN** a user has active sort or filter state and opens the view modal
- **THEN** the generated YAML includes active sort as an ordered list, includes active filters, and excludes search state

### Requirement: Saved view writing
The system SHALL save the current runtime view configuration to `config_dir/tabview/views` from the view modal.

#### Scenario: Save loaded view
- **WHEN** a view was loaded from `/home/user/.config/tabview/views/cat-shards.yml` and the user saves from the view modal
- **THEN** the system writes the current view configuration atomically to that file after any required overwrite confirmation while preserving the header comment block and matching inline comments

#### Scenario: Save new placeholder view
- **WHEN** no view was loaded for `foo.bar.csv` and the user saves from the view modal
- **THEN** the system writes the current view configuration atomically to `~/.config/tabview/views/foo.bar.yml`

#### Scenario: Create saved view directory on save
- **WHEN** the saved views directory does not exist and the user saves from the view modal
- **THEN** the system creates the directory and writes the saved view file

#### Scenario: Ask before overwrite
- **WHEN** the target saved view file already exists and the user saves from the view modal
- **THEN** the system asks for overwrite confirmation using `y` and `n` before replacing the file

#### Scenario: Decline overwrite
- **WHEN** the target saved view file already exists and the user declines overwrite confirmation
- **THEN** the system leaves the existing file unchanged and returns to the view modal

#### Scenario: Save failure
- **WHEN** writing the saved view file fails
- **THEN** the system logs the error, reports it through the modal or footer message line, keeps the modal open, and keeps the viewer running

#### Scenario: No-view disables saving
- **WHEN** the user invoked `tabview --no-view data.csv`
- **THEN** saved view authoring and saving are disabled for that session

### Requirement: Non-fatal saved view failures
The system SHALL treat saved view loading, validation, matching, and application failures as non-fatal unless the user explicitly requests a missing view through `--view`.

#### Scenario: Bad view does not block data
- **WHEN** one or more saved view files are malformed
- **THEN** the system logs the failure, records a TUI warning, and still opens the requested input file if the input itself can be loaded

#### Scenario: No matching view
- **WHEN** no saved view matches the opened input
- **THEN** the system opens the input with existing default behavior and does not report an error

### Requirement: Column conditional color metadata
The system SHALL allow saved view column definitions to include conditional color formatting rules that apply to rendered cell styles without changing raw or rendered cell values.

#### Scenario: Conditional color field validates
- **WHEN** a saved view column defines valid `colors` rules using `gradient`, `match`, `range`, or `identifiers`
- **THEN** the saved view schema accepts the column definition

#### Scenario: Conditional color does not change values
- **WHEN** a conditional color rule matches a cell
- **THEN** sorting, filtering, searching, copying, and popup display continue to use the raw and rendered cell values without including style metadata

#### Scenario: Invalid conditional color is non fatal
- **WHEN** a saved view column defines an invalid conditional color rule
- **THEN** the system ignores that rule, records a non-fatal warning, and continues applying the rest of the saved view

### Requirement: Conditional color precedence
The system SHALL resolve multiple conditional color rules for a column deterministically using saved view order.

#### Scenario: First matching rule wins
- **WHEN** a cell matches more than one conditional color rule in the same column
- **THEN** the system applies the first matching rule in the column's `colors` list

#### Scenario: No matching rule
- **WHEN** a cell matches no conditional color rule
- **THEN** the cell uses the normal theme style for that row and selection state

#### Scenario: Selection preserves readability
- **WHEN** a conditionally colored cell is also the selected cell
- **THEN** the selected-cell theme background or modifier is preserved and the conditional color is applied only where it remains readable

### Requirement: Gradient conditional colors
The system SHALL support numerical `gradient` conditional colors with `mode: fixed` and `mode: auto`.

#### Scenario: Fixed gradient ranges
- **WHEN** a numeric column defines a fixed gradient with stop entries `0: green`, `50: yellow`, and `100: red`
- **THEN** values greater than or equal to `0` and less than `50` use the first stop color, values greater than or equal to `50` and less than `100` use the second stop color, and values greater than or equal to `100` use the final stop color

#### Scenario: Fixed gradient requires stops
- **WHEN** a fixed gradient omits user-defined numeric stop values
- **THEN** the system rejects that rule as invalid and records a non-fatal warning

#### Scenario: Auto gradient default steps
- **WHEN** a numeric column defines an auto gradient with two or more colors and no `steps`
- **THEN** the system distributes eight inclusive/exclusive buckets across the observed minimum and maximum parseable numeric values for that column

#### Scenario: Auto gradient custom steps
- **WHEN** a numeric column defines an auto gradient with `steps = 5`
- **THEN** the system distributes five inclusive/exclusive buckets across the observed minimum and maximum parseable numeric values for that column

#### Scenario: Auto gradient ignores non numeric values
- **WHEN** an auto gradient column contains values that cannot be parsed as numbers
- **THEN** those values are ignored when calculating the column minimum and maximum and receive no gradient color unless another rule matches

### Requirement: Match conditional colors
The system SHALL support universal `match` conditional colors for discrete values across string, number, and boolean columns.

#### Scenario: Boolean match
- **WHEN** a column defines `match` with `true: green`
- **THEN** boolean true values in that column render with green conditional styling

#### Scenario: Numeric match
- **WHEN** a column defines `match` with `0: yellow`
- **THEN** numeric zero values in that column render with yellow conditional styling

#### Scenario: String match
- **WHEN** a column defines `match` with `active: green`
- **THEN** rendered values equal to `active` under the column's type normalization render with green conditional styling

#### Scenario: Multiple match entries
- **WHEN** a column defines one `match` rule with multiple value/color entries
- **THEN** the system evaluates entries in saved-view order and applies the first matching entry color

### Requirement: Range conditional colors
The system SHALL support numerical `range` conditional colors for explicit numeric intervals where unmatched values are left uncolored.

#### Scenario: Lower bound range
- **WHEN** a percentage column defines a range entry `"<10": red`
- **THEN** parseable values lower than `10` render with red conditional styling and values greater than or equal to `10` are not colored by that rule

#### Scenario: Upper bound range
- **WHEN** a percentage column defines a range entry `">=90": red`
- **THEN** parseable values greater than or equal to `90` render with red conditional styling and values lower than `90` are not colored by that rule

#### Scenario: Bounded range
- **WHEN** a numeric column defines a range entry `">=50 <75": yellow`
- **THEN** parseable values greater than or equal to `50` and lower than `75` match that range

#### Scenario: Range leaves gaps uncolored
- **WHEN** a numeric column defines only ranges for `<10` and `>=90`
- **THEN** parseable values from `10` through values lower than `90` receive no color from those range rules

### Requirement: Identifier conditional colors
The system SHALL support string-mode `identifiers` conditional colors that assign unique rendered column values to generated colors from theme-level or view-level color families.

#### Scenario: Unique identifiers get stable colors
- **WHEN** a string or IP column defines `identifiers: {}`
- **THEN** each unique rendered value in that column receives a deterministic color reference from the active theme identifier families

#### Scenario: Repeated identifiers reuse colors
- **WHEN** a column with `identifiers: {}` contains the same rendered value in multiple rows
- **THEN** every occurrence of that value receives the same color

#### Scenario: Theme automatic identifier colors
- **WHEN** a column defines `identifiers: { colors: auto }`
- **THEN** identifier colors are generated from the active theme `[identifiers].colors` families

#### Scenario: View override identifier colors
- **WHEN** a column defines `identifiers: { colors: [cyan, "palette(198)", "#25A39AFF"] }`
- **THEN** identifier colors for that column are generated from the view-defined color families instead of the active theme families
