use std::collections::HashMap;

use unicode_width::UnicodeWidthStr;

use crate::ops::sort::{sort_rows, SortDirection, SortMode};

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

impl Viewport {
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            origin: Position::default(),
            height,
            width,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableView {
    header: Option<Vec<String>>,
    header_visible: bool,
    rows: Vec<Vec<String>>,
    cursor: Position,
    viewport: Viewport,
    mark: Option<Position>,
    column_width_mode: ColumnWidthMode,
    column_gap: usize,
    column_widths: Vec<usize>,
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

        Self {
            header_visible: header.is_some(),
            header,
            rows,
            cursor: Position::default(),
            viewport,
            mark: None,
            column_width_mode: ColumnWidthMode::Mode,
            column_gap: 2,
            column_widths: Vec::new(),
        }
    }

    pub fn with_column_width_mode(mut self, mode: ColumnWidthMode) -> Self {
        self.column_width_mode = mode;
        self
    }

    pub fn header(&self) -> Option<&[String]> {
        self.header.as_deref()
    }

    pub fn header_visible(&self) -> bool {
        self.header_visible
    }

    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
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
        self.cursor.row = row.min(self.rows.len().saturating_sub(1));
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
        self.goto(self.rows.len().saturating_sub(1), self.cursor.column);
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
        self.header
            .as_ref()
            .map(Vec::len)
            .or_else(|| self.rows.first().map(Vec::len))
            .unwrap_or(0)
    }

    pub fn sort_current_column(&mut self, mode: SortMode, direction: SortDirection) {
        sort_rows(&mut self.rows, self.cursor.column, mode, direction);
        self.keep_cursor_visible();
    }

    pub fn set_column_gap(&mut self, gap: usize) {
        self.column_gap = gap;
    }

    pub fn adjust_column_gap(&mut self, delta: isize) {
        self.column_gap = self.column_gap.saturating_add_signed(delta);
    }

    pub fn set_column_width_mode(&mut self, mode: ColumnWidthMode) {
        self.column_width_mode = mode;
        self.column_widths.clear();
    }

    pub fn toggle_variable_column_width_mode(&mut self) {
        self.column_width_mode = match self.column_width_mode {
            ColumnWidthMode::Mode => ColumnWidthMode::Max,
            ColumnWidthMode::Max | ColumnWidthMode::Fixed(_) => ColumnWidthMode::Mode,
        };
        self.column_widths.clear();
    }

    pub fn set_all_column_widths(&mut self, width: usize) {
        self.column_width_mode = ColumnWidthMode::Fixed(width.clamp(1, u16::MAX as usize) as u16);
        self.column_widths.clear();
    }

    pub fn set_current_column_width(&mut self, width: usize) {
        self.ensure_custom_column_widths();
        if let Some(column_width) = self.column_widths.get_mut(self.cursor.column) {
            *column_width = width.max(1);
        }
    }

    pub fn maximize_current_column_width(&mut self) {
        let max_widths = self.computed_column_widths(ColumnWidthMode::Max);
        if let Some(width) = max_widths.get(self.cursor.column) {
            self.set_current_column_width(*width);
        }
    }

    pub fn adjust_all_column_widths(&mut self, delta: isize) {
        self.ensure_custom_column_widths();
        for width in &mut self.column_widths {
            *width = width.saturating_add_signed(delta).max(1);
        }
    }

    pub fn adjust_current_column_width(&mut self, delta: isize) {
        self.ensure_custom_column_widths();
        if let Some(width) = self.column_widths.get_mut(self.cursor.column) {
            *width = width.saturating_add_signed(delta).max(1);
        }
    }

    pub fn effective_column_widths(&self) -> Vec<usize> {
        if self.column_widths.len() == self.column_count() {
            self.column_widths.clone()
        } else {
            self.computed_column_widths(self.column_width_mode)
        }
    }

    pub fn restore_view_settings_from(&mut self, previous: &TableView) {
        self.column_width_mode = previous.column_width_mode;
        self.column_gap = previous.column_gap;
        self.column_widths = previous.column_widths.clone();
        self.mark = previous.mark;
    }

    fn keep_cursor_visible(&mut self) {
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
        if self.column_widths.len() != self.column_count() {
            self.column_widths = self.computed_column_widths(self.column_width_mode);
        }
    }

    fn computed_column_widths(&self, mode: ColumnWidthMode) -> Vec<usize> {
        let mut rows = Vec::new();
        if let Some(header) = self.header() {
            rows.push(header.to_vec());
        }
        rows.extend(self.rows().iter().cloned());
        column_widths(&rows, mode, self.column_gap)
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
}
