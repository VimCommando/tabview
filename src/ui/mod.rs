pub mod terminal;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::command::KeyBinding;
use crate::ops::sort::{is_numeric_cell, is_numeric_placeholder};
use crate::view::TableView;

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
    let widths = view.effective_column_widths();
    let alignments = column_alignments(view);
    let left_alignments = vec![Alignment::Left; widths.len()];

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
                    column_gap: view.column_gap(),
                    alignments: &left_alignments,
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
                column_gap: view.column_gap(),
                alignments: &alignments,
            },
        );
        row_y += 1;
    }
}

pub fn render_popup(title: &str, body: &str, area: Rect, buffer: &mut Buffer) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    let popup_style = Style::default().fg(Color::White).bg(Color::Black);
    let x2 = area.x + area.width - 1;
    let y2 = area.y + area.height - 1;

    for y in area.y..=y2 {
        for x in area.x..=x2 {
            let cell = &mut buffer[(x, y)];
            cell.reset();
            cell.set_symbol(" ");
            cell.set_style(popup_style);
        }
    }

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
        popup_style.add_modifier(Modifier::BOLD),
    );
    for (offset, line) in body
        .lines()
        .take(area.height.saturating_sub(2) as usize)
        .enumerate()
    {
        buffer.set_string(area.x + 1, area.y + 1 + offset as u16, line, popup_style);
    }
}

struct RowRender<'a> {
    area: Rect,
    y: u16,
    widths: &'a [usize],
    style: Style,
    selected_column: Option<usize>,
    column_offset: usize,
    column_gap: usize,
    alignments: &'a [Alignment],
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
        let alignment = render.alignments.get(column).copied().unwrap_or_default();
        let cell = align_cell(cell, width, "…", alignment);
        buffer.set_stringn(x, render.y, &cell, width, style);
        x = x
            .saturating_add(width as u16)
            .saturating_add(render.column_gap as u16);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Alignment {
    #[default]
    Left,
    Right,
}

fn column_alignments(view: &TableView) -> Vec<Alignment> {
    (0..view.column_count())
        .map(|column| {
            let mut has_numeric_value = false;
            for row in view.rows() {
                let Some(cell) = row.get(column).map(|cell| cell.trim()) else {
                    continue;
                };
                if cell.is_empty() {
                    continue;
                }
                if is_numeric_placeholder(cell) {
                    continue;
                }
                if !is_numeric_cell(cell, view.numeric_column_profile(column)) {
                    return Alignment::Left;
                }
                has_numeric_value = true;
            }
            if has_numeric_value {
                Alignment::Right
            } else {
                Alignment::Left
            }
        })
        .collect()
}

fn visible_row_capacity(view: &TableView, area: Rect) -> usize {
    let header_height = usize::from(view.header_visible() && view.header().is_some());
    usize::from(area.height)
        .saturating_sub(2)
        .saturating_sub(header_height)
        .max(1)
}

fn visible_column_capacity(view: &TableView, area: Rect) -> usize {
    let widths = view.effective_column_widths();
    let mut used = 0usize;
    let mut columns = 0usize;
    for width in widths.iter().skip(view.viewport().origin.column) {
        let required = *width + usize::from(columns > 0) * view.column_gap();
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
    let content_width = area.width.saturating_sub(2) as usize;
    let key_width = bindings
        .iter()
        .map(|binding| UnicodeWidthStr::width(binding.keys))
        .max()
        .unwrap_or(1)
        .min(16);
    let body = if content_width >= 74 && bindings.len() > 12 {
        let split_at = bindings.len().div_ceil(2);
        let column_gap = 2;
        let column_width = (content_width - column_gap) / 2;
        (0..split_at)
            .map(|idx| {
                let left = format_binding(&bindings[idx], key_width, column_width);
                let right = bindings
                    .get(idx + split_at)
                    .map(|binding| format_binding(binding, key_width, column_width))
                    .unwrap_or_else(|| " ".repeat(column_width));
                format!("{left}{}{}", " ".repeat(column_gap), right)
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let column_width = content_width.max(1);
        bindings
            .iter()
            .map(|binding| format_binding(binding, key_width, column_width))
            .collect::<Vec<_>>()
            .join("\n")
    };
    render_popup("Help", &body, area, buffer);
}

fn format_binding(binding: &KeyBinding, key_width: usize, column_width: usize) -> String {
    let separator_width = 1;
    if column_width <= key_width + separator_width {
        return fit_text(binding.keys, column_width);
    }
    let desc_width = column_width
        .saturating_sub(key_width)
        .saturating_sub(separator_width);
    format!(
        "{} {}",
        fit_text(binding.keys, key_width),
        fit_text(binding.description, desc_width)
    )
}

fn fit_text(value: &str, width: usize) -> String {
    truncate_cell(value, width, "…")
}

fn align_cell(cell: &str, width: usize, truncation: &str, alignment: Alignment) -> String {
    if UnicodeWidthStr::width(cell) > width {
        return truncate_cell(cell, width, truncation);
    }

    match alignment {
        Alignment::Left => format!("{cell:<width$}"),
        Alignment::Right => {
            let padding = width.saturating_sub(UnicodeWidthStr::width(cell));
            format!("{}{}", " ".repeat(padding), cell)
        }
    }
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
    fn right_aligns_numeric_columns() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Value"], &["alpha", "2"], &["beta", "100"]]),
            Viewport::new(10, 2),
        );
        view.set_all_column_widths(5);
        let area = Rect::new(0, 0, 20, 6);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);

        assert_eq!(buffer[(7, 2)].symbol(), "V");
        assert_eq!(buffer[(8, 2)].symbol(), "a");
        assert_eq!(buffer[(7, 3)].symbol(), " ");
        assert_eq!(buffer[(8, 3)].symbol(), " ");
        assert_eq!(buffer[(9, 3)].symbol(), " ");
        assert_eq!(buffer[(10, 3)].symbol(), " ");
        assert_eq!(buffer[(11, 3)].symbol(), "2");
    }

    #[test]
    fn right_aligns_suffixed_numeric_columns() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Size"], &["alpha", "2MiB"], &["beta", "512kb"]]),
            Viewport::new(10, 2),
        );
        view.set_all_column_widths(6);
        let area = Rect::new(0, 0, 24, 6);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);

        assert_eq!(buffer[(8, 3)].symbol(), " ");
        assert_eq!(buffer[(9, 3)].symbol(), " ");
        assert_eq!(buffer[(10, 3)].symbol(), "2");
        assert_eq!(buffer[(11, 3)].symbol(), "M");
        assert_eq!(buffer[(12, 3)].symbol(), "i");
        assert_eq!(buffer[(13, 3)].symbol(), "B");
    }

    #[test]
    fn right_aligns_time_hint_numeric_columns() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Duration"], &["alpha", "2m"], &["beta", "30"]]),
            Viewport::new(10, 2),
        );
        view.set_all_column_widths(5);
        let area = Rect::new(0, 0, 20, 6);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);

        assert_eq!(buffer[(9, 3)].symbol(), " ");
        assert_eq!(buffer[(10, 3)].symbol(), "2");
        assert_eq!(buffer[(11, 3)].symbol(), "m");
    }

    #[test]
    fn right_aligns_percent_numeric_columns() {
        let mut view = TableView::classify(
            rows(&[&["Name", "Rate"], &["alpha", "2.5%"], &["beta", "100%"]]),
            Viewport::new(10, 2),
        );
        view.set_all_column_widths(6);
        let area = Rect::new(0, 0, 22, 6);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);

        assert_eq!(buffer[(8, 3)].symbol(), " ");
        assert_eq!(buffer[(9, 3)].symbol(), " ");
        assert_eq!(buffer[(10, 3)].symbol(), "2");
        assert_eq!(buffer[(11, 3)].symbol(), ".");
        assert_eq!(buffer[(12, 3)].symbol(), "5");
        assert_eq!(buffer[(13, 3)].symbol(), "%");
    }

    #[test]
    fn ignores_placeholders_when_aligning_numeric_columns() {
        let mut view = TableView::classify(
            rows(&[
                &["Name", "Value"],
                &["alpha", "2.5%"],
                &["beta", "null"],
                &["gamma", "N/A"],
            ]),
            Viewport::new(10, 4),
        );
        view.set_all_column_widths(6);
        let area = Rect::new(0, 0, 22, 8);
        let mut buffer = Buffer::empty(area);
        render_table(&mut view, area, &mut buffer);

        assert_eq!(buffer[(8, 3)].symbol(), " ");
        assert_eq!(buffer[(9, 3)].symbol(), " ");
        assert_eq!(buffer[(10, 3)].symbol(), "2");
        assert_eq!(buffer[(11, 3)].symbol(), ".");
        assert_eq!(buffer[(12, 3)].symbol(), "5");
        assert_eq!(buffer[(13, 3)].symbol(), "%");
        assert_eq!(buffer[(8, 4)].symbol(), " ");
        assert_eq!(buffer[(10, 4)].symbol(), "n");
        assert_eq!(buffer[(13, 5)].symbol(), "A");
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
        assert_eq!(buffer[(1, 1)].style().bg, Some(Color::Black));
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
    fn renders_help_columns_with_stable_alignment() {
        let area = Rect::new(0, 0, 78, 22);
        let mut buffer = Buffer::empty(area);
        render_help_popup(&crate::command::default_key_bindings(), area, &mut buffer);
        let text = buffer_text(&buffer);

        assert!(text.contains("PgUp/PgDn/J/K Move a page vertically"));
        assert!(!text.contains("PgUp/PgDn/J/KMove"));

        let line_with_sort = text
            .lines()
            .find(|line| line.contains("Move a page vertically"))
            .expect("page motion help line");
        assert!(line_with_sort.contains("r"));
        assert!(text.contains("s/S"));
        assert!(text.contains("Lexical sort current"));
        assert!(text.contains("a/A"));
        assert!(text.contains("Natural sort current"));
        assert!(text.contains("#/@"));
        assert!(text.contains("Numeric sort current"));
    }

    #[test]
    fn truncates_unicode_aware_cells() {
        assert_eq!(truncate_cell("abcdef", 4, "…"), "abc…");
        assert_eq!(truncate_cell("中abcdef", 4, "…"), "中a…");
    }
}
