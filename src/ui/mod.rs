pub mod terminal;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::command::KeyBinding;
use crate::view::{column_widths, ColumnWidthMode, TableView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Cell,
    Info,
    Help,
    Search,
}

pub fn render_table(view: &mut TableView, area: Rect, buffer: &mut Buffer) {
    let viewport_height = visible_row_capacity(view, area);
    let viewport_width = visible_column_capacity(view, area);
    view.resize_viewport(viewport_height, viewport_width);

    let cursor = view.cursor();
    let viewport = view.viewport();
    let location = format!(" ({},{}) ", cursor.row + 1, cursor.column + 1);
    buffer.set_string(
        area.x,
        area.y,
        &location,
        Style::default().add_modifier(Modifier::REVERSED),
    );

    if let Some(cell) = view
        .rows()
        .get(cursor.row)
        .and_then(|row| row.get(cursor.column))
    {
        buffer.set_string(
            area.x + location.len() as u16 + 1,
            area.y,
            cell,
            Style::default(),
        );
    }

    if area.height <= 1 {
        return;
    }

    for x in area.x..area.x + area.width {
        buffer[(x, area.y + 1)].set_symbol("─");
    }

    let mut row_y = area.y + 2;
    let all_rows = display_rows(view);
    let widths = column_widths(&all_rows, ColumnWidthMode::Max, 2);

    if view.header_visible() {
        if let Some(header) = view.header() {
            render_row(
                buffer,
                header,
                RowRender {
                    area,
                    y: row_y,
                    widths: &widths,
                    style: Style::default().add_modifier(Modifier::BOLD),
                    selected_column: None,
                    column_offset: viewport.origin.column,
                },
            );
            row_y += 1;
        }
    }

    for (idx, row) in view
        .rows()
        .iter()
        .enumerate()
        .skip(viewport.origin.row)
        .take(viewport.height)
    {
        if row_y >= area.y + area.height {
            break;
        }
        let selected_column = (idx == cursor.row).then_some(cursor.column);
        render_row(
            buffer,
            row,
            RowRender {
                area,
                y: row_y,
                widths: &widths,
                style: Style::default(),
                selected_column,
                column_offset: viewport.origin.column,
            },
        );
        row_y += 1;
    }
}

pub fn render_popup(title: &str, body: &str, area: Rect, buffer: &mut Buffer) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    let x2 = area.x + area.width - 1;
    let y2 = area.y + area.height - 1;

    for x in area.x..=x2 {
        buffer[(x, area.y)].set_symbol("─");
        buffer[(x, y2)].set_symbol("─");
    }
    for y in area.y..=y2 {
        buffer[(area.x, y)].set_symbol("│");
        buffer[(x2, y)].set_symbol("│");
    }
    buffer[(area.x, area.y)].set_symbol("┌");
    buffer[(x2, area.y)].set_symbol("┐");
    buffer[(area.x, y2)].set_symbol("└");
    buffer[(x2, y2)].set_symbol("┘");

    buffer.set_string(
        area.x + 1,
        area.y,
        title,
        Style::default().add_modifier(Modifier::BOLD),
    );
    for (offset, line) in body
        .lines()
        .take(area.height.saturating_sub(2) as usize)
        .enumerate()
    {
        buffer.set_string(
            area.x + 1,
            area.y + 1 + offset as u16,
            line,
            Style::default(),
        );
    }
}

fn display_rows(view: &TableView) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    if let Some(header) = view.header() {
        rows.push(header.to_vec());
    }
    rows.extend(view.rows().iter().cloned());
    rows
}

struct RowRender<'a> {
    area: Rect,
    y: u16,
    widths: &'a [usize],
    style: Style,
    selected_column: Option<usize>,
    column_offset: usize,
}

fn render_row(buffer: &mut Buffer, row: &[String], render: RowRender<'_>) {
    let mut x = render.area.x;
    for (column, cell) in row.iter().enumerate().skip(render.column_offset) {
        if x >= render.area.x + render.area.width {
            break;
        }
        let width = render.widths.get(column).copied().unwrap_or(1);
        let style = if render.selected_column == Some(column) {
            render.style.add_modifier(Modifier::REVERSED)
        } else {
            render.style
        };
        let cell = truncate_cell(cell, width, "…");
        buffer.set_stringn(x, render.y, &cell, width, style);
        x = x.saturating_add(width as u16).saturating_add(2);
    }
}

fn visible_row_capacity(view: &TableView, area: Rect) -> usize {
    let header_height = usize::from(view.header_visible() && view.header().is_some());
    usize::from(area.height)
        .saturating_sub(2)
        .saturating_sub(header_height)
        .max(1)
}

fn visible_column_capacity(view: &TableView, area: Rect) -> usize {
    let all_rows = display_rows(view);
    let widths = column_widths(&all_rows, ColumnWidthMode::Max, 2);
    let mut used = 0usize;
    let mut columns = 0usize;
    for width in widths.iter().skip(view.viewport().origin.column) {
        let required = *width + usize::from(columns > 0) * 2;
        if columns > 0 && used + required > usize::from(area.width) {
            break;
        }
        used += required;
        columns += 1;
    }
    columns.max(1).min(view.column_count().max(1))
}

pub fn render_cell_popup(cell: &str, title: &str, area: Rect, buffer: &mut Buffer) -> bool {
    if cell.is_empty() {
        return false;
    }
    render_popup(title, cell, area, buffer);
    true
}

pub fn render_info_popup(info: &str, area: Rect, buffer: &mut Buffer) {
    render_popup("Info", info, area, buffer);
}

pub fn render_help_popup(bindings: &[KeyBinding], area: Rect, buffer: &mut Buffer) {
    let body = bindings
        .iter()
        .map(|binding| format!("{:<12}{}", binding.keys, binding.description))
        .collect::<Vec<_>>()
        .join("\n");
    render_popup("Help", &body, area, buffer);
}

pub fn render_search_prompt(query: &str, area: Rect, buffer: &mut Buffer) {
    render_popup("Search", &format!("Search: {query}"), area, buffer);
}

pub fn truncate_cell(cell: &str, width: usize, truncation: &str) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(cell) <= width {
        return format!("{cell:<width$}");
    }

    let truncation_width = UnicodeWidthStr::width(truncation);
    if truncation_width >= width {
        return truncation.chars().take(1).collect();
    }

    let target = width - truncation_width;
    let mut rendered = String::new();
    let mut used = 0;
    for ch in cell.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if used + ch_width > target {
            break;
        }
        rendered.push(ch);
        used += ch_width;
    }
    rendered.push_str(&" ".repeat(target.saturating_sub(used)));
    rendered.push_str(truncation);
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Viewport;

    fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let area = buffer.area;
        (area.y..area.y + area.height)
            .map(|y| {
                (area.x..area.x + area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_initial_header_layout() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "1"]]),
            Viewport::new(10, 4),
        );
        let area = Rect::new(0, 0, 24, 6);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("(1,1)"));
        assert!(text.contains("Name"));
        assert!(text.contains("alpha"));
    }

    #[test]
    fn renders_without_header_when_not_classified() {
        let mut view = TableView::classify(rows(&[&["1", "2"], &["3", "4"]]), Viewport::new(10, 4));
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("1"));
        assert!(text.contains("3"));
    }

    #[test]
    fn renders_from_viewport_origin() {
        let mut view = TableView::classify(
            rows(&[
                &["A", "B", "C"],
                &["r1c1", "r1c2", "r1c3"],
                &["r2c1", "r2c2", "r2c3"],
                &["r3c1", "r3c2", "r3c3"],
            ]),
            Viewport::new(1, 1),
        );
        view.goto(2, 2);
        let area = Rect::new(0, 0, 16, 4);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("(3,3)"));
        assert!(text.contains("r3c3"));
        assert!(!text.contains("r1c1"));
    }

    #[test]
    fn renders_popup_box() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        render_popup("Cell", "contents", area, &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("Cell"));
        assert!(text.contains("contents"));
        assert!(text.contains("┌"));
    }

    #[test]
    fn empty_cell_popup_is_noop() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        assert!(!render_cell_popup("", "Cell", area, &mut buffer));
    }

    #[test]
    fn renders_help_from_bindings_and_search_prompt() {
        let area = Rect::new(0, 0, 40, 8);
        let mut buffer = Buffer::empty(area);
        render_help_popup(&crate::command::default_key_bindings(), area, &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("Move selection"));

        let mut buffer = Buffer::empty(area);
        render_search_prompt("abc", area, &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("Search: abc"));
    }

    #[test]
    fn truncates_unicode_aware_cells() {
        assert_eq!(truncate_cell("abcdef", 4, "…"), "abc…");
        assert_eq!(truncate_cell("中abcdef", 4, "…"), "中a…");
    }
}
