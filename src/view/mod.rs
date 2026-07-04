mod column;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use unicode_width::UnicodeWidthStr;

use crate::ops::filter::{ActiveFilter, FilterCondition, FilterKind, FilterMode, FilterParseError};
use crate::ops::sort::{
    parse_bool_key, parse_numeric_scalar, sort_rows_by_specs, NumericColumnProfile, SortDirection,
    SortMode, SortSpec,
};
#[cfg(feature = "saved-views")]
use crate::theme::ConditionalValue;
use crate::theme::{identifier_color_ref, ConditionalColorRule};
use column::{ColumnIndex, Columns};

const MAX_ACTIVE_SORT_KEYS: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColumnWidthMode {
    #[default]
    Mode,
    Max,
    Fixed(u16),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub origin: Position,
    pub height: usize,
    pub width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlignment {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSortKey {
    pub column: usize,
    pub mode: SortMode,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnTypeChoice {
    Text,
    Date,
    Ip,
    Float,
    Integer,
    SemVer,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnFormatChoice {
    Plain,
    Locale,
    Uppercase,
    Lowercase,
    Char,
    Bit,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnSortChoice {
    None,
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnFilterSummary {
    pub mode: &'static str,
    pub kind: &'static str,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    pub visible_column: usize,
    pub source_column: usize,
    pub name: String,
    pub visible: bool,
    pub alignment: Option<ColumnAlignment>,
    pub column_type: ColumnTypeChoice,
    pub format: ColumnFormatChoice,
    pub sort: ColumnSortChoice,
    pub filters: Vec<ColumnFilterSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfoUpdate {
    pub visible: bool,
    pub alignment: Option<ColumnAlignment>,
    pub column_type: ColumnTypeChoice,
    pub format: ColumnFormatChoice,
    pub sort: ColumnSortChoice,
    pub clear_filters: bool,
}

#[allow(
    dead_code,
    reason = "saved-views metadata variants are feature-applied"
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ColumnTypeMetadata {
    #[default]
    Text,
    Date,
    Ip,
    Float,
    Int,
    SemVer,
    BooleanWord,
    BooleanChar,
    BooleanBit,
}

#[allow(dead_code, reason = "saved-views display variants are feature-applied")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DisplayFormatMetadata {
    #[default]
    Plain,
    Locale,
    Mask,
    Uppercase,
    Lowercase,
    BooleanChar,
    BooleanBit,
    BooleanWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NumberMaskMetadata {
    grouped: bool,
    decimal_places: usize,
}

impl NumberMaskMetadata {
    #[cfg(feature = "saved-views")]
    fn to_mask(self) -> String {
        let mut mask = if self.grouped {
            "#,##0".to_owned()
        } else {
            "0".to_owned()
        };
        if self.decimal_places > 0 {
            mask.push('.');
            mask.push_str(&"0".repeat(self.decimal_places));
        }
        mask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocaleMetadata {
    grouping_separator: char,
    decimal_separator: char,
}

impl Default for LocaleMetadata {
    fn default() -> Self {
        Self::en_us()
    }
}

impl LocaleMetadata {
    fn en_us() -> Self {
        Self {
            grouping_separator: ',',
            decimal_separator: '.',
        }
    }

    #[cfg(feature = "saved-views")]
    fn from_posix(value: Option<&str>) -> Self {
        let value = value
            .map(str::to_owned)
            .or_else(system_locale)
            .unwrap_or_else(|| "en_US".to_owned());
        let language = value.split(['.', '@']).next().unwrap_or("en_US");
        match language {
            "de_DE" | "es_ES" | "it_IT" | "nl_NL" => Self {
                grouping_separator: '.',
                decimal_separator: ',',
            },
            "fr_FR" | "fr_BE" => Self {
                grouping_separator: ' ',
                decimal_separator: ',',
            },
            _ => Self::en_us(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ColumnDisplayMetadata {
    column_type: ColumnTypeMetadata,
    format: DisplayFormatMetadata,
    mask: Option<NumberMaskMetadata>,
    locale: LocaleMetadata,
}

#[derive(Debug, Clone, Default)]
struct ColumnColorMetadata {
    numeric_min_max: Option<(f64, f64)>,
    identifier_indexes: BTreeMap<String, usize>,
}

impl Viewport {
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            origin: Position::default(),
            height,
            width,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableView {
    header: Option<Vec<String>>,
    header_visible: bool,
    rows: Vec<Vec<String>>,
    visible_rows: Vec<usize>,
    filters: Vec<ActiveFilter>,
    cursor: Position,
    viewport: Viewport,
    mark: Option<Position>,
    column_width_mode: ColumnWidthMode,
    column_gap: usize,
    column_widths: Vec<usize>,
    computed_column_widths_cache: Vec<usize>,
    column_width_modified: BTreeSet<usize>,
    hidden_columns: BTreeSet<usize>,
    column_alignment_overrides: Vec<Option<ColumnAlignment>>,
    column_display: Vec<ColumnDisplayMetadata>,
    column_color_rules: Vec<Vec<ConditionalColorRule>>,
    column_color_metadata: Vec<ColumnColorMetadata>,
    column_metadata_modified: BTreeSet<usize>,
    sort_keys: Vec<ActiveSortKey>,
    columns: Columns,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadState {
    pub cursor: Position,
    pub viewport_origin: Position,
    pub column_width_mode: ColumnWidthMode,
    pub column_gap: usize,
    pub column_widths: Vec<usize>,
    pub search: Option<String>,
}

impl ReloadState {
    pub fn capture(
        view: &TableView,
        column_width_mode: ColumnWidthMode,
        column_gap: usize,
        column_widths: Vec<usize>,
        search: Option<String>,
    ) -> Self {
        Self {
            cursor: view.cursor(),
            viewport_origin: view.viewport().origin,
            column_width_mode,
            column_gap,
            column_widths,
            search,
        }
    }

    pub fn apply_to(&self, view: &mut TableView) {
        view.viewport.origin = self.viewport_origin;
        view.goto(self.cursor.row, self.cursor.column);
    }
}

impl TableView {
    pub fn classify(rows: Vec<Vec<String>>, viewport: Viewport) -> Self {
        let has_header = rows.len() > 1
            && rows
                .first()
                .is_some_and(|row| !row.iter().any(|cell| cell.parse::<f64>().is_ok()));

        let (header, rows) = if has_header {
            let mut rows = rows;
            (Some(rows.remove(0)), rows)
        } else {
            (None, rows)
        };

        let columns = Columns::infer(header.as_deref(), &rows);
        let visible_rows = (0..rows.len()).collect();

        Self {
            header_visible: header.is_some(),
            header,
            rows,
            visible_rows,
            filters: Vec::new(),
            cursor: Position::default(),
            viewport,
            mark: None,
            column_width_mode: ColumnWidthMode::Mode,
            column_gap: 2,
            column_widths: Vec::new(),
            computed_column_widths_cache: Vec::new(),
            column_width_modified: BTreeSet::new(),
            hidden_columns: BTreeSet::new(),
            column_alignment_overrides: vec![None; columns.len()],
            column_display: vec![ColumnDisplayMetadata::default(); columns.len()],
            column_color_rules: vec![Vec::new(); columns.len()],
            column_color_metadata: vec![ColumnColorMetadata::default(); columns.len()],
            column_metadata_modified: BTreeSet::new(),
            sort_keys: Vec::new(),
            columns,
        }
    }

    pub fn with_column_width_mode(mut self, mode: ColumnWidthMode) -> Self {
        self.column_width_mode = mode;
        self
    }

    pub fn header(&self) -> Option<&[String]> {
        self.header.as_deref()
    }

    pub fn rendered_header(&self) -> Option<Vec<String>> {
        self.rendered_source_header().map(|header| {
            header
                .iter()
                .enumerate()
                .filter(|(source_column, _)| self.source_column_visible(*source_column))
                .map(|(_, cell)| cell.clone())
                .collect()
        })
    }

    fn rendered_source_header(&self) -> Option<Vec<String>> {
        self.header.as_ref().map(|header| {
            header
                .iter()
                .enumerate()
                .map(|(source_column, cell)| {
                    format!(
                        "{}{}{cell}",
                        self.source_column_sort_indicator(source_column)
                            .unwrap_or_default(),
                        self.source_column_filter_indicator(source_column)
                            .unwrap_or_default()
                    )
                })
                .collect()
        })
    }

    pub fn header_visible(&self) -> bool {
        self.header_visible
    }

    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.visible_rows.len()
    }

    pub fn total_row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn visible_rows(&self) -> impl Iterator<Item = &Vec<String>> {
        self.visible_rows
            .iter()
            .filter_map(|row_idx| self.rows.get(*row_idx))
    }

    pub fn visible_rows_vec(&self) -> Vec<Vec<String>> {
        self.visible_rows()
            .map(|row| self.visible_cells_for_row(row))
            .collect()
    }

    pub fn rendered_visible_row(&self, row: usize) -> Option<Vec<String>> {
        self.visible_row(row)
            .map(|row| self.visible_cells_for_row(row))
    }

    pub fn visible_raw_rows_vec(&self) -> Vec<Vec<String>> {
        self.visible_rows()
            .map(|row| {
                self.visible_source_columns()
                    .into_iter()
                    .map(|source_column| row.get(source_column).cloned().unwrap_or_default())
                    .collect()
            })
            .collect()
    }

    pub fn search_rows_vec(&self) -> Vec<Vec<String>> {
        self.visible_rows()
            .map(|row| {
                self.visible_source_columns()
                    .into_iter()
                    .map(|source_column| {
                        let raw = row
                            .get(source_column)
                            .map(String::as_str)
                            .unwrap_or_default();
                        let rendered = self.render_source_cell(source_column, Some(raw));
                        if rendered == raw {
                            raw.to_owned()
                        } else {
                            format!("{raw}\n{rendered}")
                        }
                    })
                    .collect()
            })
            .collect()
    }

    fn visible_cells_for_row(&self, row: &[String]) -> Vec<String> {
        self.visible_source_columns()
            .into_iter()
            .map(|source_column| {
                self.render_source_cell(source_column, row.get(source_column).map(String::as_str))
            })
            .collect()
    }

    pub fn current_cell(&self) -> Option<&str> {
        let source_column = self.source_column_for_visible(self.cursor.column)?;
        self.visible_row(self.cursor.row)?
            .get(source_column)
            .map(String::as_str)
    }

    pub fn current_raw_cell(&self) -> Option<&str> {
        self.current_cell()
    }

    pub fn current_cell_rendered(&self) -> Option<String> {
        let source_column = self.source_column_for_visible(self.cursor.column)?;
        let raw = self
            .visible_row(self.cursor.row)?
            .get(source_column)
            .map(String::as_str);
        Some(self.render_source_cell(source_column, raw))
    }

    pub fn current_cell_matches(&self, query: &str) -> bool {
        let Some(source_column) = self.source_column_for_visible(self.cursor.column) else {
            return false;
        };
        let Some(raw) = self
            .visible_row(self.cursor.row)
            .and_then(|row| row.get(source_column).map(String::as_str))
        else {
            return false;
        };
        let query = query.to_lowercase();
        raw.to_lowercase().contains(&query)
            || self
                .render_source_cell(source_column, Some(raw))
                .to_lowercase()
                .contains(&query)
    }

    pub fn conditional_color_for_visible_cell(
        &self,
        row: usize,
        visible_column: usize,
    ) -> Option<String> {
        let source_column = self.source_column_for_visible(visible_column)?;
        let rules = self.column_color_rules.get(source_column)?;
        if rules.is_empty() {
            return None;
        }
        let source_row = self.source_row_for_visible_row(row)?;
        let raw = self
            .rows
            .get(source_row)
            .and_then(|row| row.get(source_column).map(String::as_str))
            .unwrap_or_default();
        let rendered = self.render_source_cell(source_column, Some(raw));
        let numeric = parse_numeric_scalar(raw, self.source_numeric_column_profile(source_column));
        let metadata = self.column_color_metadata.get(source_column);
        let min_max = metadata.and_then(|metadata| metadata.numeric_min_max);

        rules.iter().find_map(|rule| match rule {
            ConditionalColorRule::Identifiers { colors } => metadata
                .and_then(|metadata| metadata.identifier_indexes.get(&rendered).copied())
                .map(|index| identifier_color_ref(index, colors)),
            _ => rule.color_for(raw, &rendered, numeric, min_max),
        })
    }

    pub fn default_cell_style_token_for_visible_column(
        &self,
        visible_column: usize,
    ) -> &'static str {
        let Some(source_column) = self.source_column_for_visible(visible_column) else {
            return "table.cell";
        };
        match self
            .column_display
            .get(source_column)
            .map(|metadata| metadata.column_type)
            .unwrap_or_default()
        {
            ColumnTypeMetadata::BooleanWord
            | ColumnTypeMetadata::BooleanChar
            | ColumnTypeMetadata::BooleanBit => "table.cell.boolean",
            ColumnTypeMetadata::Float | ColumnTypeMetadata::Int | ColumnTypeMetadata::SemVer => {
                "table.cell.number"
            }
            _ if self.columns.is_numeric(ColumnIndex::new(source_column)) => "table.cell.number",
            _ => "table.cell.string",
        }
    }

    pub fn visible_row(&self, row: usize) -> Option<&Vec<String>> {
        self.visible_rows
            .get(row)
            .and_then(|row_idx| self.rows.get(*row_idx))
    }

    pub fn source_row_for_visible_row(&self, row: usize) -> Option<usize> {
        self.visible_rows.get(row).copied()
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn column_gap(&self) -> usize {
        self.column_gap
    }

    pub fn column_width_mode(&self) -> ColumnWidthMode {
        self.column_width_mode
    }

    pub(crate) fn is_numeric_column(&self, column: usize) -> bool {
        self.source_column_for_visible(column)
            .is_some_and(|source_column| self.columns.is_numeric(ColumnIndex::new(source_column)))
    }

    fn source_numeric_column_profile(&self, source_column: usize) -> NumericColumnProfile {
        self.columns
            .numeric_profile(ColumnIndex::new(source_column))
    }

    pub(crate) fn filter_kind_enabled(&self, column: usize, kind: FilterKind) -> bool {
        kind != FilterKind::Numeric || self.is_numeric_column(column)
    }

    pub(crate) fn default_filter_kind(&self, column: usize) -> FilterKind {
        if self.is_numeric_column(column) {
            FilterKind::Numeric
        } else {
            FilterKind::Text
        }
    }

    pub fn filtered_columns(&self) -> Vec<usize> {
        let mut columns = self
            .filters
            .iter()
            .filter_map(|filter| self.visible_column_for_source(filter.column))
            .collect::<Vec<_>>();
        columns.sort_unstable();
        columns.dedup();
        columns
    }

    pub fn column_has_filter(&self, column: usize) -> bool {
        self.source_column_for_visible(column)
            .is_some_and(|source_column| self.source_column_has_filter(source_column))
    }

    fn source_column_has_filter(&self, source_column: usize) -> bool {
        self.filters
            .iter()
            .any(|filter| filter.column == source_column)
    }

    fn source_column_filter_indicator(&self, source_column: usize) -> Option<&'static str> {
        let mut filters = self
            .filters
            .iter()
            .filter(|filter| filter.column == source_column);
        let first = filters.next()?;
        if filters.next().is_some() {
            return Some("±");
        }
        Some(match first.mode {
            FilterMode::In => "+",
            FilterMode::Out => "-",
        })
    }

    fn source_column_sort_indicator(&self, source_column: usize) -> Option<&'static str> {
        self.sort_keys
            .iter()
            .find(|key| key.column == source_column)
            .map(|key| match key.direction {
                SortDirection::Ascending => "▲",
                SortDirection::Descending => "▼",
            })
    }

    pub fn mark(&self) -> Option<Position> {
        self.mark
    }

    pub fn resize_viewport(&mut self, height: usize, width: usize) {
        self.viewport.height = height.max(1);
        self.viewport.width = width.max(1);
        self.keep_cursor_visible();
    }

    pub fn toggle_header(&mut self) {
        if self.header.is_some() {
            self.header_visible = !self.header_visible;
        }
    }

    pub fn goto(&mut self, row: usize, column: usize) {
        self.cursor.row = row.min(self.visible_rows.len().saturating_sub(1));
        self.cursor.column = column.min(self.column_count().saturating_sub(1));
        self.keep_cursor_visible();
    }

    pub fn move_by(&mut self, row_delta: isize, column_delta: isize) {
        let row = self.cursor.row.saturating_add_signed(row_delta);
        let column = self.cursor.column.saturating_add_signed(column_delta);
        self.goto(row, column);
    }

    pub fn page_by(&mut self, row_pages: isize, column_pages: isize, count: usize) {
        let count = count.max(1) as isize;
        let row_delta = row_pages * self.viewport.height.max(1) as isize * count;
        let column_delta = column_pages * self.viewport.width.max(1) as isize * count;
        self.move_by(row_delta, column_delta);
    }

    pub fn goto_top(&mut self) {
        self.goto(0, self.cursor.column);
    }

    pub fn goto_bottom(&mut self) {
        self.goto(
            self.visible_rows.len().saturating_sub(1),
            self.cursor.column,
        );
    }

    pub fn goto_user_row(&mut self, row: usize) {
        self.goto(row.saturating_sub(1), self.cursor.column);
    }

    pub fn goto_user_column(&mut self, column: usize) {
        self.goto(self.cursor.row, column.saturating_sub(1));
    }

    pub fn set_mark(&mut self) {
        self.mark = Some(self.cursor);
    }

    pub fn goto_mark(&mut self) {
        if let Some(mark) = self.mark {
            self.goto(mark.row, mark.column);
        }
    }

    pub fn column_count(&self) -> usize {
        self.visible_source_columns().len()
    }

    pub fn source_column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn hidden_column_count(&self) -> usize {
        self.hidden_columns.len()
    }

    #[cfg(feature = "saved-views")]
    pub fn is_numeric_source_column(&self, source_column: usize) -> bool {
        self.columns.is_numeric(ColumnIndex::new(source_column))
    }

    #[cfg(feature = "saved-views")]
    pub fn type_sort_mode_for_source(&self, source_column: usize) -> SortMode {
        self.sort_mode_for_source(source_column, SortMode::Lexical)
    }

    pub fn sort_current_column(&mut self, mode: SortMode, direction: SortDirection) {
        let Some(column) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        let mode = self.sort_mode_for_source(column, mode);
        self.activate_sort_key(column, mode, direction);
        self.computed_column_widths_cache.clear();
        self.apply_active_sorts();
        self.recompute_visible_rows();
        self.keep_cursor_visible();
    }

    pub fn clear_current_column_sort(&mut self) {
        let Some(column) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        self.sort_keys.retain(|key| key.column != column);
        self.computed_column_widths_cache.clear();
        self.apply_active_sorts();
        self.recompute_visible_rows();
        self.keep_cursor_visible();
    }

    pub fn sort_keys(&self) -> &[ActiveSortKey] {
        &self.sort_keys
    }

    fn activate_sort_key(&mut self, column: usize, mode: SortMode, direction: SortDirection) {
        if self.sort_keys.first().is_some_and(|key| {
            key.column == column && key.mode == mode && key.direction == direction
        }) {
            self.sort_keys.remove(0);
            return;
        }

        self.sort_keys.retain(|key| key.column != column);
        self.sort_keys.insert(
            0,
            ActiveSortKey {
                column,
                mode,
                direction,
            },
        );
        self.sort_keys.truncate(MAX_ACTIVE_SORT_KEYS);
    }

    fn apply_active_sorts(&mut self) {
        let specs = self
            .sort_keys
            .iter()
            .map(|key| SortSpec {
                column: key.column,
                mode: key.mode,
                direction: key.direction,
                numeric_profile: self.source_numeric_column_profile(key.column),
            })
            .collect::<Vec<_>>();
        sort_rows_by_specs(&mut self.rows, &specs);
    }

    fn sort_mode_for_source(&self, source_column: usize, requested: SortMode) -> SortMode {
        if requested != SortMode::Lexical {
            return requested;
        }
        match self
            .column_display
            .get(source_column)
            .map(|metadata| metadata.column_type)
            .unwrap_or_default()
        {
            #[cfg(feature = "saved-views")]
            ColumnTypeMetadata::Date => SortMode::Date,
            #[cfg(feature = "saved-views")]
            ColumnTypeMetadata::Ip => SortMode::Ip,
            #[cfg(feature = "saved-views")]
            ColumnTypeMetadata::SemVer => SortMode::SemVer,
            #[cfg(feature = "saved-views")]
            ColumnTypeMetadata::BooleanWord
            | ColumnTypeMetadata::BooleanChar
            | ColumnTypeMetadata::BooleanBit => SortMode::Boolean,
            _ => requested,
        }
    }

    pub(crate) fn apply_filter(
        &mut self,
        column: usize,
        mode: FilterMode,
        kind: FilterKind,
        input: String,
    ) -> Result<(), FilterParseError> {
        let Some(source_column) = self.source_column_for_visible(column) else {
            return Ok(());
        };
        if kind == FilterKind::Numeric && !self.is_numeric_column(column) {
            return Err(FilterParseError::NumericUnavailable);
        }
        let condition = FilterCondition::parse(
            kind,
            &input,
            self.source_numeric_column_profile(source_column),
        )?;
        self.filters.push(ActiveFilter::new(
            source_column,
            mode,
            kind,
            input,
            condition,
        ));
        self.computed_column_widths_cache.clear();
        self.recompute_visible_rows();
        Ok(())
    }

    pub fn clear_filters_for_column(&mut self, column: usize) {
        if let Some(source_column) = self.source_column_for_visible(column) {
            self.filters.retain(|filter| filter.column != source_column);
        }
        self.computed_column_widths_cache.clear();
        self.recompute_visible_rows();
    }

    pub fn set_column_gap(&mut self, gap: usize) {
        self.column_gap = gap;
        self.computed_column_widths_cache.clear();
    }

    pub fn adjust_column_gap(&mut self, delta: isize) {
        self.column_gap = self.column_gap.saturating_add_signed(delta);
        self.computed_column_widths_cache.clear();
    }

    pub fn set_column_width_mode(&mut self, mode: ColumnWidthMode) {
        self.column_width_mode = mode;
        self.column_widths.clear();
        self.computed_column_widths_cache.clear();
        self.column_width_modified.clear();
    }

    pub fn toggle_variable_column_width_mode(&mut self) {
        self.column_width_mode = match self.column_width_mode {
            ColumnWidthMode::Mode => ColumnWidthMode::Max,
            ColumnWidthMode::Max | ColumnWidthMode::Fixed(_) => ColumnWidthMode::Mode,
        };
        self.column_widths.clear();
        self.computed_column_widths_cache.clear();
        self.column_width_modified.clear();
    }

    pub fn set_all_column_widths(&mut self, width: usize) {
        self.column_width_mode = ColumnWidthMode::Fixed(width.clamp(1, u16::MAX as usize) as u16);
        self.column_widths.clear();
        self.computed_column_widths_cache.clear();
        self.column_width_modified = (0..self.source_column_count()).collect();
    }

    pub fn set_current_column_width(&mut self, width: usize) {
        self.ensure_custom_column_widths();
        let Some(source_column) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        if let Some(column_width) = self.column_widths.get_mut(source_column) {
            *column_width = width.max(1);
            self.column_width_modified.insert(source_column);
        }
    }

    pub fn maximize_current_column_width(&mut self) {
        let max_widths = self.computed_column_widths(ColumnWidthMode::Max);
        let Some(source_column) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        if let Some(width) = max_widths.get(source_column) {
            self.set_current_column_width(*width);
        }
    }

    pub fn adjust_all_column_widths(&mut self, delta: isize) {
        self.ensure_custom_column_widths();
        for width in &mut self.column_widths {
            *width = width.saturating_add_signed(delta).max(1);
        }
        self.column_width_modified = (0..self.source_column_count()).collect();
    }

    pub fn adjust_current_column_width(&mut self, delta: isize) {
        self.ensure_custom_column_widths();
        let Some(source_column) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        if let Some(width) = self.column_widths.get_mut(source_column) {
            *width = width.saturating_add_signed(delta).max(1);
            self.column_width_modified.insert(source_column);
        }
    }

    pub fn effective_column_widths(&self) -> Vec<usize> {
        let source_widths = if self.column_widths.len() == self.source_column_count() {
            self.column_widths.clone()
        } else {
            self.computed_column_widths(self.column_width_mode)
        };
        self.visible_source_columns()
            .into_iter()
            .map(|source_column| source_widths.get(source_column).copied().unwrap_or(1))
            .collect()
    }

    pub fn effective_column_widths_cached(&mut self) -> Vec<usize> {
        if self.column_widths.len() == self.source_column_count() {
            let source_widths = &self.column_widths;
            return self
                .visible_source_columns_iter()
                .map(|source_column| source_widths.get(source_column).copied().unwrap_or(1))
                .collect();
        }
        if self.computed_column_widths_cache.len() != self.source_column_count() {
            self.computed_column_widths_cache = self.computed_column_widths(self.column_width_mode);
        }
        let source_widths = &self.computed_column_widths_cache;
        self.visible_source_columns_iter()
            .map(|source_column| source_widths.get(source_column).copied().unwrap_or(1))
            .collect()
    }

    pub fn column_alignment_override(&self, column: usize) -> Option<ColumnAlignment> {
        let source_column = self.source_column_for_visible(column)?;
        self.column_alignment_overrides
            .get(source_column)
            .copied()
            .flatten()
    }

    pub fn column_alignment(&self, column: usize) -> ColumnAlignment {
        let Some(source_column) = self.source_column_for_visible(column) else {
            return ColumnAlignment::Left;
        };
        if let Some(alignment) = self
            .column_alignment_overrides
            .get(source_column)
            .copied()
            .flatten()
        {
            return alignment;
        }
        match self
            .column_display
            .get(source_column)
            .map(|metadata| metadata.column_type)
            .unwrap_or_default()
        {
            ColumnTypeMetadata::Float | ColumnTypeMetadata::Int => ColumnAlignment::Right,
            _ if self.columns.is_numeric(ColumnIndex::new(source_column)) => ColumnAlignment::Right,
            _ => ColumnAlignment::Left,
        }
    }

    pub fn current_column_info(&self) -> Option<ColumnInfo> {
        let visible_column = self.cursor.column;
        let source_column = self.source_column_for_visible(visible_column)?;
        let display = self
            .column_display
            .get(source_column)
            .copied()
            .unwrap_or_default();
        let sort = self
            .sort_keys
            .iter()
            .find(|key| key.column == source_column)
            .map(|key| match key.direction {
                SortDirection::Ascending => ColumnSortChoice::Ascending,
                SortDirection::Descending => ColumnSortChoice::Descending,
            })
            .unwrap_or(ColumnSortChoice::None);
        let filters = self
            .filters
            .iter()
            .filter(|filter| filter.column == source_column)
            .map(|filter| ColumnFilterSummary {
                mode: match filter.mode {
                    FilterMode::In => "in",
                    FilterMode::Out => "out",
                },
                kind: match filter.kind {
                    FilterKind::Text => "text",
                    FilterKind::Regex => "regex",
                    FilterKind::Numeric => "numeric",
                },
                input: filter.input.clone(),
            })
            .collect();
        Some(ColumnInfo {
            visible_column,
            source_column,
            name: self.source_column_name(source_column),
            visible: self.source_column_visible(source_column),
            alignment: self
                .column_alignment_overrides
                .get(source_column)
                .copied()
                .flatten(),
            column_type: column_type_choice(display.column_type),
            format: column_format_choice(display.format),
            sort,
            filters,
        })
    }

    pub fn apply_current_column_info(&mut self, update: ColumnInfoUpdate) {
        let Some(source_column) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };

        if update.visible {
            self.hidden_columns.remove(&source_column);
        } else if self.column_count() > 1 {
            self.hidden_columns.insert(source_column);
        }

        if let Some(slot) = self.column_alignment_overrides.get_mut(source_column) {
            *slot = update.alignment;
        }
        if let Some(display) = self.column_display.get_mut(source_column) {
            display.column_type = column_type_metadata_from_choice(update.column_type);
            display.format = display_format_metadata_from_choice(update.format);
            display.mask = None;
        }
        self.column_metadata_modified.insert(source_column);

        if update.clear_filters {
            self.filters.retain(|filter| filter.column != source_column);
        }

        self.sort_keys.retain(|key| key.column != source_column);
        match update.sort {
            ColumnSortChoice::None => {}
            ColumnSortChoice::Ascending | ColumnSortChoice::Descending => {
                self.activate_sort_key(
                    source_column,
                    self.sort_mode_for_source(source_column, SortMode::Lexical),
                    match update.sort {
                        ColumnSortChoice::Ascending => SortDirection::Ascending,
                        ColumnSortChoice::Descending => SortDirection::Descending,
                        ColumnSortChoice::None => unreachable!(),
                    },
                );
            }
        }

        self.computed_column_widths_cache.clear();
        self.apply_active_sorts();
        self.recompute_visible_rows();
        self.keep_cursor_visible();
    }

    pub fn restore_view_settings_from(&mut self, previous: &TableView) {
        self.column_width_mode = previous.column_width_mode;
        self.column_gap = previous.column_gap;
        self.column_widths = previous.column_widths.clone();
        self.computed_column_widths_cache.clear();
        self.column_width_modified = previous.column_width_modified.clone();
        self.hidden_columns = previous.hidden_columns.clone();
        let source_column_count = self.source_column_count();
        self.hidden_columns
            .retain(|source_column| *source_column < source_column_count);
        if self.hidden_columns.len() >= source_column_count {
            self.hidden_columns.clear();
        }
        self.column_alignment_overrides = previous.column_alignment_overrides.clone();
        self.column_alignment_overrides
            .resize(source_column_count, None);
        self.column_display = previous.column_display.clone();
        self.column_display
            .resize(source_column_count, ColumnDisplayMetadata::default());
        self.column_color_rules = previous.column_color_rules.clone();
        self.column_color_rules
            .resize(source_column_count, Vec::new());
        self.rebuild_column_color_metadata();
        self.column_metadata_modified = previous.column_metadata_modified.clone();
        self.column_metadata_modified
            .retain(|source_column| *source_column < source_column_count);
        self.sort_keys = previous
            .sort_keys
            .iter()
            .copied()
            .filter(|key| key.column < source_column_count)
            .collect();
        self.mark = previous.mark;
        self.filters = previous.filters.clone();
        self.apply_active_sorts();
        self.recompute_visible_rows();
    }

    pub fn hide_current_column(&mut self) {
        if self.column_count() <= 1 {
            return;
        }
        if let Some(source_column) = self.source_column_for_visible(self.cursor.column) {
            self.hidden_columns.insert(source_column);
            self.keep_cursor_visible();
        }
    }

    pub fn hide_columns_left(&mut self, count: usize) {
        let count = count.max(1);
        let current = self.cursor.column;
        let start = current.saturating_sub(count);
        let to_hide = (start..current)
            .filter_map(|column| self.source_column_for_visible(column))
            .collect::<Vec<_>>();
        for source_column in to_hide {
            self.hidden_columns.insert(source_column);
        }
        self.goto(
            self.cursor.row,
            start.min(self.column_count().saturating_sub(1)),
        );
    }

    pub fn hide_columns_right(&mut self, count: usize) {
        let count = count.max(1);
        let current = self.cursor.column;
        let to_hide = (current + 1..=current.saturating_add(count))
            .filter_map(|column| self.source_column_for_visible(column))
            .collect::<Vec<_>>();
        for source_column in to_hide {
            self.hidden_columns.insert(source_column);
        }
        self.keep_cursor_visible();
    }

    pub fn show_hidden_left(&mut self, count: usize) {
        let Some(current_source) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        for (shown, source_column) in (0..current_source).rev().enumerate() {
            if shown >= count.max(1) || !self.hidden_columns.contains(&source_column) {
                break;
            }
            self.hidden_columns.remove(&source_column);
        }
        self.cursor.column = self
            .visible_column_for_source(current_source)
            .unwrap_or(self.cursor.column);
        self.keep_cursor_visible();
    }

    pub fn show_hidden_right(&mut self, count: usize) {
        let Some(current_source) = self.source_column_for_visible(self.cursor.column) else {
            return;
        };
        for (shown, source_column) in (current_source + 1..self.source_column_count()).enumerate() {
            if shown >= count.max(1) || !self.hidden_columns.contains(&source_column) {
                break;
            }
            self.hidden_columns.remove(&source_column);
        }
        self.keep_cursor_visible();
    }

    pub fn hidden_boundary_before(&self, column: usize) -> bool {
        let visible_columns = self.visible_source_columns();
        let Some(&source_column) = visible_columns.get(column) else {
            return false;
        };
        let previous_source = column
            .checked_sub(1)
            .and_then(|previous| visible_columns.get(previous).copied());
        let start = previous_source.map_or(0, |previous| previous + 1);
        (start..source_column).any(|source_column| self.hidden_columns.contains(&source_column))
    }

    pub fn hidden_boundary_after_last(&self) -> bool {
        let Some(last_visible) = self.visible_source_columns().last().copied() else {
            return false;
        };
        (last_visible + 1..self.source_column_count())
            .any(|source_column| self.hidden_columns.contains(&source_column))
    }

    #[cfg(feature = "saved-views")]
    pub fn apply_saved_columns(
        &mut self,
        resolved: &crate::saved_views::ResolvedColumns,
        locale: Option<&str>,
    ) {
        self.ensure_custom_column_widths();
        let header_widths = self.computed_header_widths();
        let content_widths = self.computed_content_widths();
        let mode_widths = self.computed_column_widths(ColumnWidthMode::Mode);
        let max_widths = self.computed_column_widths(ColumnWidthMode::Max);
        let locale = LocaleMetadata::from_posix(locale);

        for (source_column, resolved_column) in resolved.columns.iter().enumerate() {
            let Some(resolved_column) = resolved_column else {
                continue;
            };
            let column_view = &resolved_column.view;
            let inferred_column_type = if self.columns.is_numeric(ColumnIndex::new(source_column)) {
                ColumnTypeMetadata::Float
            } else {
                ColumnTypeMetadata::Text
            };
            if let Some(display) = self.column_display.get_mut(source_column) {
                display.column_type = column_view
                    .column_type
                    .map(column_type_metadata)
                    .unwrap_or(inferred_column_type);
                display.format = column_view
                    .format
                    .map(display_format_metadata)
                    .or_else(|| {
                        column_view
                            .mask
                            .as_ref()
                            .map(|_| DisplayFormatMetadata::Mask)
                    })
                    .unwrap_or(DisplayFormatMetadata::Plain);
                display.mask = column_view.mask.as_ref().map(|mask| NumberMaskMetadata {
                    grouped: mask.grouped,
                    decimal_places: mask.decimal_places,
                });
                display.locale = locale;
                if column_view.column_type.is_some()
                    || column_view.format.is_some()
                    || column_view.mask.is_some()
                {
                    self.column_metadata_modified.insert(source_column);
                }
            }
            let colors = column_view.colors.clone();
            let color_metadata = self.build_column_color_metadata(source_column, &colors);
            if let Some(slot) = self.column_color_rules.get_mut(source_column) {
                *slot = colors;
                if !slot.is_empty() {
                    self.column_metadata_modified.insert(source_column);
                }
            }
            if let Some(slot) = self.column_color_metadata.get_mut(source_column) {
                *slot = color_metadata;
            }
            if let Some(width) = column_view.width {
                if let Some(target) = self.column_widths.get_mut(source_column) {
                    *target = match width {
                        crate::saved_views::ColumnWidth::Fixed(width) => width as usize,
                        crate::saved_views::ColumnWidth::Header => {
                            header_widths.get(source_column).copied().unwrap_or(1)
                        }
                        crate::saved_views::ColumnWidth::Content => {
                            content_widths.get(source_column).copied().unwrap_or(1)
                        }
                        crate::saved_views::ColumnWidth::Mode => {
                            mode_widths.get(source_column).copied().unwrap_or(1)
                        }
                        crate::saved_views::ColumnWidth::Max => {
                            max_widths.get(source_column).copied().unwrap_or(1)
                        }
                    }
                    .max(1);
                    self.column_width_modified.insert(source_column);
                }
            }

            let alignment = column_view
                .align
                .map(|align| match align {
                    crate::saved_views::ColumnAlign::Left => ColumnAlignment::Left,
                    crate::saved_views::ColumnAlign::Right => ColumnAlignment::Right,
                })
                .or_else(|| {
                    matches!(
                        column_view.column_type,
                        Some(crate::saved_views::ColumnType::Number(_))
                    )
                    .then_some(ColumnAlignment::Right)
                });
            if let Some(alignment) = alignment {
                if let Some(slot) = self.column_alignment_overrides.get_mut(source_column) {
                    *slot = Some(alignment);
                    self.column_metadata_modified.insert(source_column);
                }
            }

            if column_view.visible == Some(false) {
                self.hidden_columns.insert(source_column);
                self.column_metadata_modified.insert(source_column);
            } else if column_view.visible == Some(true) {
                self.hidden_columns.remove(&source_column);
                self.column_metadata_modified.insert(source_column);
            }
        }
        if self.hidden_columns.len() >= self.source_column_count() {
            self.hidden_columns.clear();
        }
        self.keep_cursor_visible();
    }

    #[cfg(feature = "saved-views")]
    pub fn apply_saved_sort_keys(&mut self, sort_keys: Vec<ActiveSortKey>) {
        self.sort_keys = sort_keys
            .into_iter()
            .filter(|key| key.column < self.source_column_count())
            .take(MAX_ACTIVE_SORT_KEYS)
            .collect();
        self.computed_column_widths_cache.clear();
        self.apply_active_sorts();
        self.recompute_visible_rows();
    }

    #[cfg(feature = "saved-views")]
    pub(crate) fn apply_source_filter(
        &mut self,
        source_column: usize,
        mode: FilterMode,
        kind: FilterKind,
        input: String,
    ) -> Result<(), FilterParseError> {
        if source_column >= self.source_column_count() {
            return Ok(());
        }
        if kind == FilterKind::Numeric && !self.columns.is_numeric(ColumnIndex::new(source_column))
        {
            return Err(FilterParseError::NumericUnavailable);
        }
        let condition = FilterCondition::parse(
            kind,
            &input,
            self.source_numeric_column_profile(source_column),
        )?;
        self.filters.push(ActiveFilter::new(
            source_column,
            mode,
            kind,
            input,
            condition,
        ));
        self.computed_column_widths_cache.clear();
        self.recompute_visible_rows();
        Ok(())
    }

    #[cfg(feature = "saved-views")]
    pub fn to_saved_view_yaml(
        &self,
        name: &str,
        input_filename: &str,
        locale: Option<&str>,
    ) -> String {
        let mut yaml = String::new();
        yaml.push_str(&format!("name: {}\n", yaml_scalar(name)));
        if let Some(locale) = locale {
            yaml.push_str(&format!("locale: {}\n", yaml_scalar(locale)));
        }
        yaml.push_str("filenames:\n");
        yaml.push_str(&format!("  - {}\n", yaml_scalar(input_filename)));

        let mut column_blocks = Vec::new();
        for source_column in 0..self.source_column_count() {
            let include_column = self.column_width_modified.contains(&source_column)
                || self.hidden_columns.contains(&source_column)
                || self
                    .column_alignment_overrides
                    .get(source_column)
                    .is_some_and(Option::is_some)
                || self.column_metadata_modified.contains(&source_column)
                || self
                    .column_color_rules
                    .get(source_column)
                    .is_some_and(|rules| !rules.is_empty());
            if !include_column {
                continue;
            }
            let key = self.source_column_name(source_column);
            let mut block = format!("  {}:\n", yaml_key(&key));
            if self.column_metadata_modified.contains(&source_column) {
                let metadata = self
                    .column_display
                    .get(source_column)
                    .copied()
                    .unwrap_or_default();
                if metadata.column_type != ColumnTypeMetadata::Text {
                    block.push_str(&format!(
                        "    type: {}\n",
                        column_type_metadata_name(metadata.column_type)
                    ));
                }
                if metadata.format != DisplayFormatMetadata::Plain {
                    block.push_str(&format!(
                        "    format: {}\n",
                        display_format_metadata_name(metadata.format)
                    ));
                }
                if let Some(mask) = metadata.mask {
                    block.push_str(&format!("    mask: {}\n", yaml_scalar(&mask.to_mask())));
                }
            }
            if self.column_width_modified.contains(&source_column) {
                let width = self
                    .column_widths
                    .get(source_column)
                    .copied()
                    .unwrap_or(1)
                    .max(1);
                block.push_str(&format!("    width: {width}\n"));
            }
            if self.hidden_columns.contains(&source_column) {
                block.push_str("    visible: false\n");
            }
            if let Some(alignment) = self
                .column_alignment_overrides
                .get(source_column)
                .copied()
                .flatten()
            {
                let align = match alignment {
                    ColumnAlignment::Left => "left",
                    ColumnAlignment::Right => "right",
                };
                block.push_str(&format!("    align: {align}\n"));
            }
            if let Some(rules) = self.column_color_rules.get(source_column) {
                if !rules.is_empty() {
                    block.push_str("    colors:\n");
                    for rule in rules {
                        push_color_rule_yaml(&mut block, rule);
                    }
                }
            }
            column_blocks.push(block);
        }
        if !column_blocks.is_empty() {
            yaml.push_str("columns:\n");
            for block in column_blocks {
                yaml.push_str(&block);
            }
        }

        if !self.sort_keys.is_empty() {
            yaml.push_str("sort:\n");
            for key in &self.sort_keys {
                yaml.push_str(&format!(
                    "  - column: {}\n",
                    yaml_scalar(&self.source_column_name(key.column))
                ));
                yaml.push_str(&format!(
                    "    direction: {}\n",
                    match key.direction {
                        SortDirection::Ascending => "asc",
                        SortDirection::Descending => "desc",
                    }
                ));
                yaml.push_str(&format!("    kind: {}\n", sort_mode_name(key.mode)));
            }
        }

        if !self.filters.is_empty() {
            yaml.push_str("filters:\n");
            for filter in &self.filters {
                yaml.push_str(&format!(
                    "  - column: {}\n",
                    yaml_scalar(&self.source_column_name(filter.column))
                ));
                yaml.push_str(&format!(
                    "    action: {}\n",
                    match filter.mode {
                        FilterMode::In => "in",
                        FilterMode::Out => "out",
                    }
                ));
                yaml.push_str(&format!("    kind: {}\n", filter_kind_name(filter.kind)));
                yaml.push_str(&format!("    condition: {}\n", yaml_scalar(&filter.input)));
            }
        }
        yaml
    }

    fn source_column_name(&self, source_column: usize) -> String {
        self.header
            .as_ref()
            .and_then(|header| header.get(source_column))
            .cloned()
            .unwrap_or_else(|| format!("column_{}", source_column + 1))
    }

    fn keep_cursor_visible(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .min(self.visible_rows.len().saturating_sub(1));
        self.cursor.column = self
            .cursor
            .column
            .min(self.column_count().saturating_sub(1));
        if self.cursor.row < self.viewport.origin.row {
            self.viewport.origin.row = self.cursor.row;
        } else if self.cursor.row >= self.viewport.origin.row + self.viewport.height {
            self.viewport.origin.row = self.cursor.row + 1 - self.viewport.height;
        }

        if self.cursor.column < self.viewport.origin.column {
            self.viewport.origin.column = self.cursor.column;
        } else if self.cursor.column >= self.viewport.origin.column + self.viewport.width {
            self.viewport.origin.column = self.cursor.column + 1 - self.viewport.width;
        }
    }

    fn ensure_custom_column_widths(&mut self) {
        if self.column_widths.len() != self.source_column_count() {
            self.column_widths = self.computed_column_widths(self.column_width_mode);
        }
        self.computed_column_widths_cache.clear();
    }

    fn computed_column_widths(&self, mode: ColumnWidthMode) -> Vec<usize> {
        if let ColumnWidthMode::Fixed(width) = mode {
            return vec![width as usize; self.source_column_count()];
        }

        let mut rows = Vec::new();
        if let Some(header) = self.rendered_source_header() {
            rows.push(header);
        }
        rows.extend(self.rendered_source_rows());
        column_widths(&rows, mode, self.column_gap)
    }

    #[cfg(feature = "saved-views")]
    fn computed_header_widths(&self) -> Vec<usize> {
        if let Some(header) = self.rendered_source_header() {
            header
                .iter()
                .map(|cell| UnicodeWidthStr::width(cell.as_str()).max(1))
                .collect()
        } else {
            vec![1; self.source_column_count()]
        }
    }

    #[cfg(feature = "saved-views")]
    fn computed_content_widths(&self) -> Vec<usize> {
        let mut rows = Vec::new();
        if let Some(header) = self.rendered_source_header() {
            rows.push(header);
        }
        rows.extend(self.rendered_source_rows());
        column_widths(&rows, ColumnWidthMode::Max, self.column_gap)
    }

    fn rendered_source_rows(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| {
                (0..self.source_column_count())
                    .map(|source_column| {
                        self.render_source_cell(
                            source_column,
                            row.get(source_column).map(String::as_str),
                        )
                    })
                    .collect()
            })
            .collect()
    }

    fn recompute_visible_rows(&mut self) {
        self.visible_rows = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(row_idx, row)| self.row_passes_filters(row).then_some(row_idx))
            .collect();
        self.keep_cursor_visible();
    }

    fn row_passes_filters(&self, row: &[String]) -> bool {
        self.filters.iter().all(|filter| {
            let raw = row
                .get(filter.column)
                .map(String::as_str)
                .unwrap_or_default();
            let rendered = self.render_source_cell(filter.column, Some(raw));
            filter.accepts_values(
                raw,
                &rendered,
                self.source_numeric_column_profile(filter.column),
            )
        })
    }

    fn source_column_visible(&self, source_column: usize) -> bool {
        !self.hidden_columns.contains(&source_column)
    }

    fn visible_source_columns(&self) -> Vec<usize> {
        self.visible_source_columns_iter().collect()
    }

    fn visible_source_columns_iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.source_column_count())
            .filter(|source_column| self.source_column_visible(*source_column))
    }

    fn source_column_for_visible(&self, column: usize) -> Option<usize> {
        self.visible_source_columns_iter().nth(column)
    }

    fn visible_column_for_source(&self, source_column: usize) -> Option<usize> {
        self.visible_source_columns_iter()
            .position(|candidate| candidate == source_column)
    }

    fn render_source_cell(&self, source_column: usize, raw: Option<&str>) -> String {
        let raw = raw.unwrap_or_default();
        let metadata = self
            .column_display
            .get(source_column)
            .copied()
            .unwrap_or_default();
        match metadata.format {
            DisplayFormatMetadata::Plain => raw.to_owned(),
            DisplayFormatMetadata::Uppercase => raw.to_uppercase(),
            DisplayFormatMetadata::Lowercase => raw.to_lowercase(),
            DisplayFormatMetadata::Locale => self
                .format_locale_number(raw, source_column, metadata.locale)
                .unwrap_or_else(|| raw.to_owned()),
            DisplayFormatMetadata::Mask => metadata
                .mask
                .and_then(|mask| self.format_masked_number(raw, source_column, mask))
                .unwrap_or_else(|| raw.to_owned()),
            DisplayFormatMetadata::BooleanChar => parse_bool_key(raw)
                .map(|value| if value { "y" } else { "n" }.to_owned())
                .unwrap_or_else(|| raw.to_owned()),
            DisplayFormatMetadata::BooleanBit => parse_bool_key(raw)
                .map(|value| if value { "1" } else { "0" }.to_owned())
                .unwrap_or_else(|| raw.to_owned()),
            DisplayFormatMetadata::BooleanWord => parse_bool_key(raw)
                .map(|value| if value { "true" } else { "false" }.to_owned())
                .unwrap_or_else(|| raw.to_owned()),
        }
    }

    fn format_locale_number(
        &self,
        raw: &str,
        source_column: usize,
        locale: LocaleMetadata,
    ) -> Option<String> {
        let value = parse_numeric_scalar(raw, self.source_numeric_column_profile(source_column))?;
        let decimal_places = raw
            .split_once('.')
            .map(|(_, fraction)| {
                fraction
                    .chars()
                    .take_while(|ch| ch.is_ascii_digit())
                    .count()
            })
            .unwrap_or(0);
        Some(format_number_parts(value, decimal_places, true, locale))
    }

    fn format_masked_number(
        &self,
        raw: &str,
        source_column: usize,
        mask: NumberMaskMetadata,
    ) -> Option<String> {
        let value = parse_numeric_scalar(raw, self.source_numeric_column_profile(source_column))?;
        Some(format_number_parts(
            value,
            mask.decimal_places,
            mask.grouped,
            LocaleMetadata::en_us(),
        ))
    }

    fn rebuild_column_color_metadata(&mut self) {
        self.column_color_metadata = (0..self.source_column_count())
            .map(|source_column| {
                let rules = self
                    .column_color_rules
                    .get(source_column)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                self.build_column_color_metadata(source_column, rules)
            })
            .collect();
    }

    fn build_column_color_metadata(
        &self,
        source_column: usize,
        rules: &[ConditionalColorRule],
    ) -> ColumnColorMetadata {
        let numeric_min_max = rules
            .iter()
            .any(|rule| matches!(rule, ConditionalColorRule::AutoGradient { .. }))
            .then(|| self.numeric_min_max(source_column))
            .flatten();
        let identifier_indexes = rules
            .iter()
            .any(|rule| matches!(rule, ConditionalColorRule::Identifiers { .. }))
            .then(|| self.identifier_indexes(source_column))
            .unwrap_or_default();
        ColumnColorMetadata {
            numeric_min_max,
            identifier_indexes,
        }
    }

    fn numeric_min_max(&self, source_column: usize) -> Option<(f64, f64)> {
        let profile = self.source_numeric_column_profile(source_column);
        let mut values = self
            .rows
            .iter()
            .filter_map(|row| row.get(source_column))
            .filter_map(|value| parse_numeric_scalar(value, profile))
            .filter(|value| value.is_finite());
        let first = values.next()?;
        Some(values.fold((first, first), |(min, max), value| {
            (min.min(value), max.max(value))
        }))
    }

    fn identifier_indexes(&self, source_column: usize) -> BTreeMap<String, usize> {
        let mut values = self
            .rows
            .iter()
            .filter_map(|row| row.get(source_column).map(String::as_str))
            .map(|raw| self.render_source_cell(source_column, Some(raw)))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
            .into_iter()
            .enumerate()
            .map(|(index, value)| (value, index))
            .collect()
    }
}

fn format_number_parts(
    value: f64,
    decimal_places: usize,
    grouped: bool,
    locale: LocaleMetadata,
) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let negative = value.is_sign_negative();
    let value = value.abs();
    let fixed = format!("{value:.decimal_places$}");
    let (integer, fraction) = fixed.split_once('.').unwrap_or((fixed.as_str(), ""));
    let mut rendered = String::new();
    if negative {
        rendered.push('-');
    }
    if grouped {
        rendered.push_str(&group_integer(integer, locale.grouping_separator));
    } else {
        rendered.push_str(integer);
    }
    if decimal_places > 0 {
        rendered.push(locale.decimal_separator);
        rendered.push_str(fraction);
    }
    rendered
}

fn group_integer(value: &str, separator: char) -> String {
    let mut grouped = String::new();
    for (idx, ch) in value.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(separator);
        }
        grouped.push(ch);
    }
    grouped.chars().rev().collect()
}

fn column_type_choice(metadata: ColumnTypeMetadata) -> ColumnTypeChoice {
    match metadata {
        ColumnTypeMetadata::Text => ColumnTypeChoice::Text,
        ColumnTypeMetadata::Date => ColumnTypeChoice::Date,
        ColumnTypeMetadata::Ip => ColumnTypeChoice::Ip,
        ColumnTypeMetadata::Float => ColumnTypeChoice::Float,
        ColumnTypeMetadata::Int => ColumnTypeChoice::Integer,
        ColumnTypeMetadata::SemVer => ColumnTypeChoice::SemVer,
        ColumnTypeMetadata::BooleanWord
        | ColumnTypeMetadata::BooleanChar
        | ColumnTypeMetadata::BooleanBit => ColumnTypeChoice::Boolean,
    }
}

fn column_type_metadata_from_choice(choice: ColumnTypeChoice) -> ColumnTypeMetadata {
    match choice {
        ColumnTypeChoice::Text => ColumnTypeMetadata::Text,
        ColumnTypeChoice::Date => ColumnTypeMetadata::Date,
        ColumnTypeChoice::Ip => ColumnTypeMetadata::Ip,
        ColumnTypeChoice::Float => ColumnTypeMetadata::Float,
        ColumnTypeChoice::Integer => ColumnTypeMetadata::Int,
        ColumnTypeChoice::SemVer => ColumnTypeMetadata::SemVer,
        ColumnTypeChoice::Boolean => ColumnTypeMetadata::BooleanWord,
    }
}

fn column_format_choice(metadata: DisplayFormatMetadata) -> ColumnFormatChoice {
    match metadata {
        DisplayFormatMetadata::Plain | DisplayFormatMetadata::Mask => ColumnFormatChoice::Plain,
        DisplayFormatMetadata::Locale => ColumnFormatChoice::Locale,
        DisplayFormatMetadata::Uppercase => ColumnFormatChoice::Uppercase,
        DisplayFormatMetadata::Lowercase => ColumnFormatChoice::Lowercase,
        DisplayFormatMetadata::BooleanChar => ColumnFormatChoice::Char,
        DisplayFormatMetadata::BooleanBit => ColumnFormatChoice::Bit,
        DisplayFormatMetadata::BooleanWord => ColumnFormatChoice::Word,
    }
}

fn display_format_metadata_from_choice(choice: ColumnFormatChoice) -> DisplayFormatMetadata {
    match choice {
        ColumnFormatChoice::Plain => DisplayFormatMetadata::Plain,
        ColumnFormatChoice::Locale => DisplayFormatMetadata::Locale,
        ColumnFormatChoice::Uppercase => DisplayFormatMetadata::Uppercase,
        ColumnFormatChoice::Lowercase => DisplayFormatMetadata::Lowercase,
        ColumnFormatChoice::Char => DisplayFormatMetadata::BooleanChar,
        ColumnFormatChoice::Bit => DisplayFormatMetadata::BooleanBit,
        ColumnFormatChoice::Word => DisplayFormatMetadata::BooleanWord,
    }
}

#[cfg(feature = "saved-views")]
fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_NUMERIC", "LANG"]
        .into_iter()
        .find_map(|key| {
            let value = std::env::var(key).ok()?;
            (!value.is_empty() && value != "C" && value != "POSIX").then_some(value)
        })
}

#[cfg(feature = "saved-views")]
fn column_type_metadata(column_type: crate::saved_views::ColumnType) -> ColumnTypeMetadata {
    match column_type {
        crate::saved_views::ColumnType::String(kind) => match kind {
            crate::saved_views::StringKind::Text => ColumnTypeMetadata::Text,
            crate::saved_views::StringKind::Date => ColumnTypeMetadata::Date,
            crate::saved_views::StringKind::Ip => ColumnTypeMetadata::Ip,
        },
        crate::saved_views::ColumnType::Number(kind) => match kind {
            crate::saved_views::NumberKind::Float => ColumnTypeMetadata::Float,
            crate::saved_views::NumberKind::Int => ColumnTypeMetadata::Int,
            crate::saved_views::NumberKind::SemVer => ColumnTypeMetadata::SemVer,
        },
        crate::saved_views::ColumnType::Boolean(kind) => match kind {
            crate::saved_views::BooleanKind::Char => ColumnTypeMetadata::BooleanChar,
            crate::saved_views::BooleanKind::Bit => ColumnTypeMetadata::BooleanBit,
            crate::saved_views::BooleanKind::Word => ColumnTypeMetadata::BooleanWord,
        },
    }
}

#[cfg(feature = "saved-views")]
fn display_format_metadata(format: crate::saved_views::DisplayFormat) -> DisplayFormatMetadata {
    match format {
        crate::saved_views::DisplayFormat::Plain => DisplayFormatMetadata::Plain,
        crate::saved_views::DisplayFormat::Locale => DisplayFormatMetadata::Locale,
        crate::saved_views::DisplayFormat::Mask => DisplayFormatMetadata::Mask,
        crate::saved_views::DisplayFormat::Uppercase => DisplayFormatMetadata::Uppercase,
        crate::saved_views::DisplayFormat::Lowercase => DisplayFormatMetadata::Lowercase,
        crate::saved_views::DisplayFormat::Char => DisplayFormatMetadata::BooleanChar,
        crate::saved_views::DisplayFormat::Bit => DisplayFormatMetadata::BooleanBit,
        crate::saved_views::DisplayFormat::Word => DisplayFormatMetadata::BooleanWord,
    }
}

#[cfg(feature = "saved-views")]
fn column_type_metadata_name(column_type: ColumnTypeMetadata) -> &'static str {
    match column_type {
        ColumnTypeMetadata::Text => "text",
        ColumnTypeMetadata::Date => "date",
        ColumnTypeMetadata::Ip => "ip",
        ColumnTypeMetadata::Float => "float",
        ColumnTypeMetadata::Int => "integer",
        ColumnTypeMetadata::SemVer => "semver",
        ColumnTypeMetadata::BooleanWord => "word",
        ColumnTypeMetadata::BooleanChar => "char",
        ColumnTypeMetadata::BooleanBit => "bit",
    }
}

#[cfg(feature = "saved-views")]
fn display_format_metadata_name(format: DisplayFormatMetadata) -> &'static str {
    match format {
        DisplayFormatMetadata::Plain => "plain",
        DisplayFormatMetadata::Locale => "locale",
        DisplayFormatMetadata::Mask => "mask",
        DisplayFormatMetadata::Uppercase => "uppercase",
        DisplayFormatMetadata::Lowercase => "lowercase",
        DisplayFormatMetadata::BooleanChar => "char",
        DisplayFormatMetadata::BooleanBit => "bit",
        DisplayFormatMetadata::BooleanWord => "word",
    }
}

#[cfg(feature = "saved-views")]
fn sort_mode_name(mode: SortMode) -> &'static str {
    match mode {
        SortMode::Lexical => "lexical",
        SortMode::Natural => "natural",
        SortMode::Numeric => "numeric",
        SortMode::Date | SortMode::SemVer | SortMode::Ip | SortMode::Boolean => "type",
    }
}

#[cfg(feature = "saved-views")]
fn filter_kind_name(kind: FilterKind) -> &'static str {
    match kind {
        FilterKind::Text => "text",
        FilterKind::Regex => "regex",
        FilterKind::Numeric => "numeric",
    }
}

#[cfg(feature = "saved-views")]
fn push_color_rule_yaml(block: &mut String, rule: &ConditionalColorRule) {
    match rule {
        ConditionalColorRule::Match { entries } => {
            block.push_str("      - match:\n");
            for entry in entries {
                block.push_str(&format!(
                    "          {}: {}\n",
                    conditional_value_yaml(&entry.value),
                    yaml_scalar(&entry.color)
                ));
            }
        }
        ConditionalColorRule::Range { entries } => {
            block.push_str("      - range:\n");
            for entry in entries {
                block.push_str(&format!(
                    "          {}: {}\n",
                    yaml_scalar(&range_entry_expression(entry)),
                    yaml_scalar(&entry.color)
                ));
            }
        }
        ConditionalColorRule::FixedGradient { stops } => {
            block.push_str("      - gradient:\n");
            block.push_str("          mode: fixed\n");
            block.push_str("          stops:\n");
            for stop in stops {
                block.push_str(&format!(
                    "            {}: {}\n",
                    yaml_scalar(&stop.value.to_string()),
                    yaml_scalar(&stop.color)
                ));
            }
        }
        ConditionalColorRule::AutoGradient { colors, steps } => {
            block.push_str("      - gradient:\n");
            block.push_str("          mode: auto\n");
            block.push_str(&format!("          steps: {steps}\n"));
            block.push_str("          colors:\n");
            for color in colors {
                block.push_str(&format!("            - {}\n", yaml_scalar(color)));
            }
        }
        ConditionalColorRule::Identifiers { colors } => {
            block.push_str("      - identifiers:\n");
            match colors {
                crate::theme::IdentifierColors::Auto => {
                    block.push_str("          colors: auto\n");
                }
                crate::theme::IdentifierColors::Colors(colors) => {
                    block.push_str("          colors:\n");
                    for color in colors {
                        block.push_str(&format!("            - {}\n", yaml_scalar(color)));
                    }
                }
            }
        }
    }
}

#[cfg(feature = "saved-views")]
fn range_entry_expression(entry: &crate::theme::RangeEntry) -> String {
    let mut parts = Vec::new();
    if let Some(value) = entry.gte {
        parts.push(format!(">={value}"));
    }
    if let Some(value) = entry.gt {
        parts.push(format!(">{value}"));
    }
    if let Some(value) = entry.lte {
        parts.push(format!("<={value}"));
    }
    if let Some(value) = entry.lt {
        parts.push(format!("<{value}"));
    }
    parts.join(" ")
}

#[cfg(feature = "saved-views")]
fn conditional_value_yaml(value: &ConditionalValue) -> String {
    match value {
        ConditionalValue::Bool(value) => value.to_string(),
        ConditionalValue::Number(value) => value.to_string(),
        ConditionalValue::String(value) => yaml_scalar(value),
    }
}

#[cfg(feature = "saved-views")]
fn yaml_key(value: &str) -> String {
    yaml_scalar(value)
}

#[cfg(feature = "saved-views")]
fn yaml_scalar(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
        && !matches!(value, "true" | "false" | "yes" | "no" | "null")
    {
        value.to_owned()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

pub fn column_widths(rows: &[Vec<String>], mode: ColumnWidthMode, gap: usize) -> Vec<usize> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    match mode {
        ColumnWidthMode::Fixed(width) => vec![width as usize; column_count],
        ColumnWidthMode::Max => (0..column_count)
            .map(|column| {
                rows.iter()
                    .filter_map(|row| row.get(column))
                    .map(|cell| UnicodeWidthStr::width(cell.as_str()))
                    .max()
                    .unwrap_or(1)
                    .clamp(1, 250)
            })
            .collect(),
        ColumnWidthMode::Mode => (0..column_count)
            .map(|column| mode_width(rows, column, gap))
            .collect(),
    }
}

fn mode_width(rows: &[Vec<String>], column: usize, gap: usize) -> usize {
    let widths: Vec<usize> = rows
        .iter()
        .filter_map(|row| row.get(column))
        .map(|cell| UnicodeWidthStr::width(cell.as_str()))
        .collect();
    if widths.is_empty() {
        return 1;
    }

    let mut counts = HashMap::<usize, usize>::new();
    for width in &widths {
        *counts.entry(*width).or_default() += 1;
    }

    let mode = counts
        .into_iter()
        .filter(|(width, _)| *width != 0)
        .max_by_key(|(_, count)| *count)
        .map(|(width, _)| width)
        .unwrap_or(0);
    let max_width = widths.into_iter().max().unwrap_or(1).max(1);
    let diff = mode.abs_diff(max_width);
    if diff > gap * 2 && diff * 10 > max_width {
        mode.max(gap).max(1)
    } else {
        max_width.max(gap).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::filter::{FilterKind, FilterMode};

    fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect()
    }

    #[test]
    fn classifies_non_numeric_first_row_as_header() {
        let view = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "1"]]),
            Viewport::new(10, 4),
        );
        assert_eq!(view.header().expect("header"), ["Name", "Value"]);
        assert!(view.header_visible());
        assert_eq!(view.rows(), rows(&[&["alpha", "1"]]));
    }

    #[test]
    fn keeps_numeric_first_row_as_data() {
        let view = TableView::classify(rows(&[&["1", "2"], &["3", "4"]]), Viewport::new(10, 4));
        assert!(view.header().is_none());
        assert_eq!(view.rows(), rows(&[&["1", "2"], &["3", "4"]]));
    }

    #[test]
    fn toggles_header_structurally() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["Name", "Value"]]),
            Viewport::new(10, 4),
        );
        view.toggle_header();
        assert!(!view.header_visible());
        assert_eq!(view.header().expect("header"), ["Name", "Value"]);
        assert_eq!(view.rows(), rows(&[&["Name", "Value"]]));
    }

    #[test]
    fn keeps_cursor_inside_table_and_viewport() {
        let mut view = TableView::classify(
            rows(&[&["A", "B"], &["1", "2"], &["3", "4"]]),
            Viewport::new(1, 1),
        );
        view.goto(10, 10);
        assert_eq!(view.cursor(), Position { row: 1, column: 1 });
        assert_eq!(view.viewport().origin, Position { row: 1, column: 1 });
    }

    #[test]
    fn computes_fixed_and_max_widths() {
        let rows = rows(&[&["a", "wide"], &["bb", "中"]]);
        assert_eq!(column_widths(&rows, ColumnWidthMode::Fixed(3), 2), [3, 3]);
        assert_eq!(column_widths(&rows, ColumnWidthMode::Max, 2), [2, 4]);
    }

    #[test]
    fn captures_and_applies_reload_state() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "1"], &["beta", "2"]]),
            Viewport::new(1, 1),
        );
        view.goto(1, 1);
        let state = ReloadState::capture(
            &view,
            ColumnWidthMode::Mode,
            2,
            vec![5, 6],
            Some("beta".to_owned()),
        );
        let mut reloaded = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "1"], &["beta", "2"]]),
            Viewport::new(1, 1),
        );
        state.apply_to(&mut reloaded);
        assert_eq!(reloaded.cursor(), Position { row: 1, column: 1 });
        assert_eq!(state.search.as_deref(), Some("beta"));
    }

    #[test]
    fn resizes_viewport_and_keeps_cursor_visible() {
        let mut view = TableView::classify(
            rows(&[
                &["A", "B", "C"],
                &["1", "2", "3"],
                &["4", "5", "6"],
                &["7", "8", "9"],
            ]),
            Viewport::new(3, 3),
        );
        view.goto(2, 2);
        view.resize_viewport(1, 1);
        assert_eq!(view.viewport().origin, Position { row: 2, column: 2 });
    }

    #[test]
    fn sorts_data_rows_by_current_column() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["b", "10"], &["a", "2"]]),
            Viewport::new(10, 2),
        );
        view.goto(0, 0);
        view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);
        assert_eq!(view.rows(), rows(&[&["a", "2"], &["b", "10"]]));
    }

    #[test]
    fn page_motion_uses_viewport_size() {
        let mut view = TableView::classify(
            rows(&[
                &["A"],
                &["1"],
                &["2"],
                &["3"],
                &["4"],
                &["5"],
                &["6"],
                &["7"],
            ]),
            Viewport::new(3, 1),
        );
        view.page_by(1, 0, 2);
        assert_eq!(view.cursor().row, 6);
        view.page_by(-1, 0, 1);
        assert_eq!(view.cursor().row, 3);
    }

    #[test]
    fn mark_round_trips_cursor_position() {
        let mut view = TableView::classify(
            rows(&[&["A", "B"], &["1", "2"], &["3", "4"]]),
            Viewport::new(3, 2),
        );
        view.goto(1, 1);
        view.set_mark();
        view.goto(0, 0);
        view.goto_mark();
        assert_eq!(view.cursor(), Position { row: 1, column: 1 });
    }

    #[test]
    fn column_width_controls_update_effective_widths() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "1"]]),
            Viewport::new(3, 2),
        );
        view.set_all_column_widths(4);
        assert_eq!(view.effective_column_widths(), [4, 4]);
        view.set_current_column_width(8);
        assert_eq!(view.effective_column_widths()[0], 8);
        view.adjust_current_column_width(-20);
        assert_eq!(view.effective_column_widths()[0], 1);
        view.adjust_column_gap(3);
        assert_eq!(view.column_gap(), 5);
    }

    #[test]
    fn cached_widths_do_not_become_explicit_widths() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "1000"]]),
            Viewport::new(3, 2),
        );

        assert!(view.column_widths.is_empty());
        assert!(view.computed_column_widths_cache.is_empty());

        assert_eq!(view.effective_column_widths_cached(), vec![5, 5]);

        assert!(view.column_widths.is_empty());
        assert_eq!(view.computed_column_widths_cache, vec![5, 5]);

        view.set_current_column_width(8);
        assert_eq!(view.column_widths[0], 8);
        assert!(view.computed_column_widths_cache.is_empty());
    }

    #[test]
    fn filter_in_keeps_matching_visible_rows_without_mutating_backing_rows() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Size"],
                &["alpha", "1gb"],
                &["beta", "2gb"],
                &["gamma", "3gb"],
            ]),
            Viewport::new(10, 2),
        );
        view.goto(0, 0);
        view.apply_filter(0, FilterMode::In, FilterKind::Text, "a".to_owned())
            .expect("apply filter");

        assert_eq!(view.row_count(), 3);
        assert_eq!(view.total_row_count(), 3);
        assert_eq!(
            view.rows(),
            rows(&[&["alpha", "1gb"], &["beta", "2gb"], &["gamma", "3gb"]])
        );
        assert_eq!(
            view.visible_rows_vec(),
            rows(&[&["alpha", "1gb"], &["beta", "2gb"], &["gamma", "3gb"]])
        );
    }

    #[test]
    fn filter_out_and_multiple_filters_reduce_visible_rows() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Size"],
                &["alpha", "1gb"],
                &["beta", "2gb"],
                &["gamma", "3gb"],
            ]),
            Viewport::new(10, 2),
        );
        view.apply_filter(0, FilterMode::Out, FilterKind::Text, "beta".to_owned())
            .expect("apply text filter");
        view.apply_filter(1, FilterMode::In, FilterKind::Numeric, ">=2gb".to_owned())
            .expect("apply numeric filter");

        assert_eq!(view.visible_rows_vec(), rows(&[&["gamma", "3gb"]]));
    }

    #[test]
    fn filters_clamp_cursor_and_can_be_cleared() {
        let mut view = TableView::classify(
            rows(&[&["Name"], &["alpha"], &["beta"], &["gamma"]]),
            Viewport::new(1, 1),
        );
        view.goto(2, 0);
        view.apply_filter(0, FilterMode::In, FilterKind::Text, "alpha".to_owned())
            .expect("apply filter");

        assert_eq!(view.cursor(), Position { row: 0, column: 0 });
        assert_eq!(view.visible_rows_vec(), rows(&[&["alpha"]]));

        view.clear_filters_for_column(0);
        assert_eq!(view.row_count(), 3);
    }

    #[test]
    fn filtered_header_indicator_participates_in_widths() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Size"], &["alpha", "1gb"]]),
            Viewport::new(10, 2),
        );
        view.apply_filter(1, FilterMode::In, FilterKind::Numeric, "<2gb".to_owned())
            .expect("apply filter");

        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["Name".to_owned(), "+Size".to_owned()]
        );
        assert!(view.effective_column_widths()[1] >= 5);
    }

    #[test]
    fn auto_widths_expand_only_when_indicators_need_space() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Size"], &["alpha", "1gb"]]),
            Viewport::new(10, 2),
        );

        assert_eq!(view.effective_column_widths(), vec![5, 4]);
        view.goto(0, 1);
        view.sort_current_column(SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["Name".to_owned(), "▲Size".to_owned()]
        );
        assert_eq!(view.effective_column_widths(), vec![5, 5]);

        view.apply_filter(1, FilterMode::In, FilterKind::Numeric, "<2gb".to_owned())
            .expect("apply filter");

        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["Name".to_owned(), "▲+Size".to_owned()]
        );
        assert_eq!(view.effective_column_widths(), vec![5, 6]);
    }

    #[test]
    fn filter_header_indicators_show_mode_and_multiples() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Size"],
                &["alpha", "1gb"],
                &["beta", "2gb"],
                &["gamma", "3gb"],
            ]),
            Viewport::new(10, 2),
        );

        view.apply_filter(0, FilterMode::Out, FilterKind::Text, "beta".to_owned())
            .expect("apply out filter");
        view.apply_filter(1, FilterMode::In, FilterKind::Numeric, ">=1gb".to_owned())
            .expect("apply in filter");
        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["-Name".to_owned(), "+Size".to_owned()]
        );

        view.apply_filter(0, FilterMode::In, FilterKind::Text, "a".to_owned())
            .expect("apply second name filter");
        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["±Name".to_owned(), "+Size".to_owned()]
        );
    }

    #[test]
    fn filtering_and_navigation_do_not_shrink_computed_widths() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Value"],
                &["short", "ok"],
                &["very-very-long", "hidden"],
            ]),
            Viewport::new(10, 2),
        );
        view.set_column_width_mode(ColumnWidthMode::Max);
        let widths_before_filter = view.effective_column_widths();

        view.apply_filter(1, FilterMode::In, FilterKind::Text, "ok".to_owned())
            .expect("apply filter");
        let widths_after_filter = view.effective_column_widths();
        view.move_by(1, 0);
        let widths_after_navigation = view.effective_column_widths();

        assert_eq!(widths_after_filter, widths_before_filter);
        assert_eq!(widths_after_navigation, widths_before_filter);
    }

    #[test]
    fn sorting_preserves_active_filters() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Size"],
                &["gamma", "3gb"],
                &["alpha", "1gb"],
                &["beta", "2gb"],
            ]),
            Viewport::new(10, 2),
        );
        view.apply_filter(1, FilterMode::In, FilterKind::Numeric, ">=2gb".to_owned())
            .expect("apply filter");
        view.goto(0, 0);
        view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);

        assert_eq!(
            view.visible_rows_vec(),
            rows(&[&["beta", "2gb"], &["gamma", "3gb"]])
        );
    }

    #[test]
    fn multi_level_sort_tracks_primary_key_and_header_markers() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Shard"],
                &["b", "2"],
                &["a", "2"],
                &["c", "1"],
                &["a", "1"],
            ]),
            Viewport::new(10, 2),
        );

        view.goto(0, 1);
        view.sort_current_column(SortMode::Numeric, SortDirection::Ascending);
        view.goto(0, 0);
        view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);

        assert_eq!(
            view.rows(),
            rows(&[&["a", "1"], &["a", "2"], &["b", "2"], &["c", "1"]])
        );
        assert_eq!(
            view.sort_keys(),
            &[
                ActiveSortKey {
                    column: 0,
                    mode: SortMode::Lexical,
                    direction: SortDirection::Ascending,
                },
                ActiveSortKey {
                    column: 1,
                    mode: SortMode::Numeric,
                    direction: SortDirection::Ascending,
                },
            ]
        );
        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["▲Name".to_owned(), "▲Shard".to_owned()]
        );

        view.clear_current_column_sort();
        assert_eq!(view.sort_keys().len(), 1);
        assert_eq!(
            view.rendered_header().expect("header"),
            vec!["Name".to_owned(), "▲Shard".to_owned()]
        );
    }

    #[test]
    fn repeated_sort_toggles_primary_key_and_keeps_last_three_sorts() {
        let mut view = TableView::classify(
            rows(&[
                &["A", "B", "C", "D"],
                &["b", "2", "x", "m"],
                &["a", "1", "y", "n"],
            ]),
            Viewport::new(10, 4),
        );

        for column in 0..4 {
            view.goto(0, column);
            view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);
        }

        assert_eq!(
            view.sort_keys()
                .iter()
                .map(|key| key.column)
                .collect::<Vec<_>>(),
            vec![3, 2, 1]
        );

        view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);
        assert_eq!(
            view.sort_keys()
                .iter()
                .map(|key| key.column)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
    }

    #[test]
    fn hidden_columns_are_omitted_from_visible_rows_and_navigation() {
        let mut view = TableView::classify(
            rows(&[&["A", "B", "C"], &["a1", "b1", "c1"]]),
            Viewport::new(10, 3),
        );
        view.goto(0, 1);
        view.hide_current_column();

        assert_eq!(view.column_count(), 2);
        assert_eq!(view.visible_rows_vec(), rows(&[&["a1", "c1"]]));
        assert_eq!(
            view.rendered_header().expect("header"),
            rows(&[&["A", "C"]])[0]
        );
        assert_eq!(view.current_cell(), Some("c1"));
        assert!(view.hidden_boundary_before(1));

        view.show_hidden_left(1);
        assert_eq!(view.column_count(), 3);
        assert_eq!(view.visible_rows_vec(), rows(&[&["a1", "b1", "c1"]]));
    }

    #[test]
    fn hide_current_column_preserves_last_visible_column() {
        let mut view = TableView::classify(rows(&[&["A"], &["a1"]]), Viewport::new(10, 1));
        view.hide_current_column();

        assert_eq!(view.column_count(), 1);
        assert_eq!(view.current_cell(), Some("a1"));
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn applies_saved_column_width_alignment_and_visibility() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Count", "Hidden"], &["alpha", "1000", "x"]]),
            Viewport::new(10, 3),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: counts
filenames: [counts.csv]
columns:
  Count:
    type: integer
    width: 12
  Hidden:
    visible: false
"#,
        )
        .expect("parse");
        let headers = view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);

        view.apply_saved_columns(&resolved, None);

        assert_eq!(view.column_count(), 2);
        assert_eq!(view.effective_column_widths(), vec![5, 12]);
        assert_eq!(
            view.column_alignment_override(1),
            Some(ColumnAlignment::Right)
        );
        assert_eq!(view.visible_rows_vec(), rows(&[&["alpha", "1000"]]));
        assert!(view.hidden_boundary_after_last());
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn applies_saved_display_formats_without_mutating_raw_rows() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Locale", "Mask", "Flag"],
                &["alpha", "1234.50", "1234.5", "yes"],
            ]),
            Viewport::new(10, 4),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r##"
name: display
locale: de_DE
filenames: [display.csv]
columns:
  Name:
    type: text
    format: uppercase
  Locale:
    type: float
    format: locale
  Mask:
    type: float
    format: mask
    mask: "#,##0.00"
  Flag:
    type: word
    format: bit
"##,
        )
        .expect("parse");
        let headers = view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);

        view.apply_saved_columns(&resolved, parsed.view.locale.as_deref());

        assert_eq!(
            view.visible_rows_vec(),
            rows(&[&["ALPHA", "1.234,50", "1,234.50", "1"]])
        );
        assert_eq!(
            view.visible_raw_rows_vec(),
            rows(&[&["alpha", "1234.50", "1234.5", "yes"]])
        );
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn search_and_text_filter_match_raw_or_rendered_values() {
        let mut view = TableView::classify(
            rows(&[&["Count"], &["1000"], &["2000"]]),
            Viewport::new(10, 1),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: counts
filenames: [counts.csv]
columns:
  Count:
    type: integer
    format: locale
"#,
        )
        .expect("parse");
        let headers = view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        view.apply_saved_columns(&resolved, Some("en_US"));

        assert_eq!(
            view.search_rows_vec(),
            rows(&[&["1000\n1,000"], &["2000\n2,000"]])
        );
        view.apply_filter(0, FilterMode::In, FilterKind::Text, "1,000".to_owned())
            .expect("filter");
        assert_eq!(view.visible_raw_rows_vec(), rows(&[&["1000"]]));
        assert_eq!(view.visible_rows_vec(), rows(&[&["1,000"]]));
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn conditional_colors_apply_without_changing_values() {
        let mut view = TableView::classify(
            rows(&[
                &["Status", "Percent"],
                &["active", "5%"],
                &["idle", "50%"],
                &["down", "95%"],
            ]),
            Viewport::new(10, 2),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: colors
filenames: [data.csv]
columns:
  Status:
    colors:
      - match:
          active: green
  Percent:
    type: number
    colors:
      - range:
          "<10": red
      - gradient:
          mode: auto
          colors: [green, yellow]
"#,
        )
        .expect("parse");
        let headers = view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        view.apply_saved_columns(&resolved, None);

        assert_eq!(
            view.conditional_color_for_visible_cell(0, 0),
            Some("green".to_owned())
        );
        assert_eq!(
            view.conditional_color_for_visible_cell(0, 1),
            Some("red".to_owned())
        );
        assert_eq!(
            view.conditional_color_for_visible_cell(1, 1),
            Some("gradient(4,8,green,yellow)".to_owned())
        );
        assert_eq!(
            view.visible_rows_vec(),
            rows(&[&["active", "5%"], &["idle", "50%"], &["down", "95%"]])
        );
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn identifier_colors_are_stable_for_unique_rendered_values() {
        let mut view = TableView::classify(
            rows(&[&["Address"], &["10.0.0.2"], &["10.0.0.1"], &["10.0.0.2"]]),
            Viewport::new(10, 1),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: identifiers
filenames: [data.csv]
columns:
  Address:
    type: ip
    colors:
      - identifiers: {}
"#,
        )
        .expect("parse");
        let headers = view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        view.apply_saved_columns(&resolved, None);

        let first = view
            .conditional_color_for_visible_cell(0, 0)
            .expect("first color");
        let second = view
            .conditional_color_for_visible_cell(1, 0)
            .expect("second color");
        let repeated = view
            .conditional_color_for_visible_cell(2, 0)
            .expect("repeated color");

        assert_ne!(first, second);
        assert_eq!(first, repeated);
        assert!(first.starts_with("identifier("));
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn restored_view_settings_preserve_conditional_color_cache() {
        let mut view = TableView::classify(
            rows(&[&["Address"], &["10.0.0.2"], &["10.0.0.1"]]),
            Viewport::new(10, 1),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: identifiers
filenames: [data.csv]
columns:
  Address:
    type: ip
    colors:
      - identifiers: {}
"#,
        )
        .expect("parse");
        let headers = view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        view.apply_saved_columns(&resolved, None);

        let mut restored = TableView::classify(
            rows(&[&["Address"], &["10.0.0.2"], &["10.0.0.1"]]),
            Viewport::new(10, 1),
        );
        restored.restore_view_settings_from(&view);

        assert_eq!(
            restored.conditional_color_for_visible_cell(0, 0),
            view.conditional_color_for_visible_cell(0, 0)
        );
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn saved_type_metadata_enables_date_semver_and_ip_sorting() {
        let mut date_view = TableView::classify(
            rows(&[
                &["Created"],
                &["2024-01-01T00:00:00Z"],
                &["2023-12-31T23:00:00Z"],
            ]),
            Viewport::new(10, 1),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: dates
filenames: [dates.csv]
columns:
  Created:
    type: date
"#,
        )
        .expect("parse");
        let headers = date_view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        date_view.apply_saved_columns(&resolved, None);
        date_view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);
        assert_eq!(
            date_view.visible_raw_rows_vec(),
            rows(&[&["2023-12-31T23:00:00Z"], &["2024-01-01T00:00:00Z"]])
        );

        let mut semver_view = TableView::classify(
            rows(&[&["Version"], &["1.10.0"], &["1.2.0"]]),
            Viewport::new(10, 1),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: versions
filenames: [versions.csv]
columns:
  Version:
    type: semver
"#,
        )
        .expect("parse");
        let headers = semver_view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        semver_view.apply_saved_columns(&resolved, None);
        semver_view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);
        assert_eq!(
            semver_view.visible_raw_rows_vec(),
            rows(&[&["1.2.0"], &["1.10.0"]])
        );

        let mut ip_view = TableView::classify(
            rows(&[&["Address"], &["2001:db8::1"], &["10.0.0.2"]]),
            Viewport::new(10, 1),
        );
        let parsed = crate::saved_views::parse_saved_view_yaml(
            r#"
name: ips
filenames: [ips.csv]
columns:
  Address:
    type: ip
"#,
        )
        .expect("parse");
        let headers = ip_view.header().expect("header").to_vec();
        let resolved = crate::saved_views::resolve_columns(&parsed.view, &headers);
        ip_view.apply_saved_columns(&resolved, None);
        ip_view.sort_current_column(SortMode::Lexical, SortDirection::Ascending);
        assert_eq!(
            ip_view.visible_raw_rows_vec(),
            rows(&[&["10.0.0.2"], &["2001:db8::1"]])
        );
    }
}
