## ADDED Requirements

### Requirement: Theme discovery
The system SHALL discover color theme TOML files from `$XDG_CONFIG_HOME/tabview/themes`, or `~/.config/tabview/themes` when `XDG_CONFIG_HOME` is unset, and SHALL provide a built-in default theme when no user theme is selected.

#### Scenario: Built-in default theme
- **WHEN** a user opens an input without selecting a theme and no theme configuration is present
- **THEN** the system applies the built-in `cmdzro` theme

#### Scenario: Discover user themes
- **WHEN** `solarized-dark.toml` exists under `~/.config/tabview/themes`
- **THEN** the system makes a theme named `solarized-dark` available for selection

#### Scenario: Missing theme directory
- **WHEN** the theme directory does not exist
- **THEN** the system opens the input using the built-in default theme without reporting an error

### Requirement: Theme selection
The system SHALL allow selecting a theme by name through configuration and SHALL fail clearly when a user explicitly selects a missing theme.

#### Scenario: Select configured theme
- **WHEN** tabview configuration selects `theme = "ops-dark"` and `ops-dark.toml` exists in the theme directory
- **THEN** the system applies the `ops-dark` theme to the TUI session

#### Scenario: Missing selected theme
- **WHEN** tabview configuration selects `theme = "missing"` and no discovered or built-in theme has that name
- **THEN** the system reports a clear configuration error and does not start the viewer

#### Scenario: Invalid unselected theme
- **WHEN** an unselected theme file is malformed
- **THEN** the system logs the failure, records a TUI warning, and continues opening the input with the selected or default theme

### Requirement: Theme TOML schema
The system SHALL ship and document a TOML theme schema covering theme metadata, color mode, palette aliases, identifier color families, and named UI style tokens.

#### Scenario: Valid theme file
- **WHEN** a theme TOML file defines `name`, `mode`, palette aliases, identifier color families, and required style tokens
- **THEN** the theme validates and can be applied

#### Scenario: Unknown style token
- **WHEN** a theme TOML file defines an unsupported style token
- **THEN** validation reports the unsupported token with the theme filename and token path

#### Scenario: Missing required token
- **WHEN** a selected theme omits a required style token
- **THEN** the system reports a clear configuration error and does not start the viewer

### Requirement: Theme identifier families
The system SHALL allow themes to define identifier color families used by saved-view `identifiers` conditional colors.

#### Scenario: Theme identifier colors
- **WHEN** a theme defines `[identifiers] colors = ["bright-green", "magenta", "cyan", "white"]`
- **THEN** `identifiers` conditional colors using `colors: auto` are generated from those families

#### Scenario: Identifier family shades
- **WHEN** an identifier family color is configured
- **THEN** the system generates 16 dark-to-light shades for that family before repeating, with the darkest shade no darker than the ANSI dark/dim foreground equivalent

#### Scenario: Built-in identifier families
- **WHEN** no user theme is selected
- **THEN** the built-in `cmdzro` theme provides green, magenta, cyan, and white identifier families

### Requirement: Color value modes
The system SHALL support 16-color names, 256-color palette indexes, and 32-bit hex colors in theme files and conditional color rules.

#### Scenario: Sixteen-color name
- **WHEN** a theme color is `green`, `bright-white`, or another supported 16-color name
- **THEN** the system maps it to the corresponding terminal color

#### Scenario: Two hundred fifty six color index
- **WHEN** a theme color is `palette(124)` or an equivalent 256-color integer notation
- **THEN** the system maps it to the corresponding 256-color terminal palette entry

#### Scenario: Thirty two bit hex color
- **WHEN** a theme color is `#25a39aFF`
- **THEN** the system parses the red, green, blue, and alpha channels and uses the RGB value for terminal rendering

#### Scenario: Lower color terminal fallback
- **WHEN** the active terminal cannot render the configured color mode
- **THEN** the system resolves each color to the nearest supported configured fallback without changing the loaded theme file

### Requirement: Cmdzro default theme
The built-in default theme SHALL use `~/.config/nvim/colors/cmdzro.vim` as its baseline while adapting the palette for table viewing constraints.

#### Scenario: Text color avoids blue
- **WHEN** default theme text, headers, ordinary cell values, and popups are rendered
- **THEN** the rendered foreground colors do not use blue-family text colors

#### Scenario: Yellow reserved for emphasis
- **WHEN** the default theme renders search highlights or emphasized UI state
- **THEN** yellow-family colors are permitted only for those search or emphasis tokens

#### Scenario: Red reserved for unhealthy state
- **WHEN** the default theme renders ordinary values, headers, or navigation UI
- **THEN** red-family colors are not used unless the token represents an error, failed validation, unhealthy status, or user-defined conditional rule

#### Scenario: Blue reserved for UI elements
- **WHEN** the default theme uses blue-family colors
- **THEN** they appear only in UI backgrounds, borders, status areas, selections, or other non-text emphasis surfaces

### Requirement: Themed Ratatui rendering
The system SHALL render Ratatui table and popup styles from the active theme tokens rather than hard-coded colors.

#### Scenario: Table chrome uses theme
- **WHEN** the table renders location, divider, header, selected cell, hidden-column marker, and footer message line
- **THEN** each element uses the corresponding active theme style token

#### Scenario: Popups use theme
- **WHEN** cell, info, help, search, filter, column info, or saved view popups are rendered
- **THEN** popup background, border, title, disabled text, active item, and action labels use the active theme tokens

#### Scenario: Search highlight uses theme
- **WHEN** search highlights are visible in the table
- **THEN** the highlighted text uses the active theme search token

### Requirement: Theme style modifiers
The system SHALL support style modifiers for theme tokens, including bold, italic, underline, reversed, and dim where supported by Ratatui and the active terminal backend.

#### Scenario: Bold header token
- **WHEN** a theme token sets `modifiers = ["bold"]` for headers
- **THEN** table headers render with Ratatui bold styling

#### Scenario: Unsupported terminal modifier
- **WHEN** the terminal backend cannot visibly render a configured modifier
- **THEN** tabview continues rendering with the configured colors and does not fail the session
