## ADDED Requirements

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
