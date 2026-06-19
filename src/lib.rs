pub mod cli;
pub mod command;
pub mod ingest;
pub mod ops;
pub mod table;
pub mod ui;
pub mod view;

use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use ops::filter::{FilterKind, FilterMode};

pub fn run(args: cli::Args) -> anyhow::Result<()> {
    let config = cli::Config::from_args(args)?;
    let source = ingest::source::InputSource::from_cli_value(&config.filename.to_string_lossy());
    let parse_options = ingest::ParseOptions {
        encoding: config.encoding,
        delimiter: config.delimiter,
        quoting: config.quoting,
        quote_char: config.quote_char as u8,
    };
    let rows = read_rows(&source, &parse_options)?;
    let mut view = view::TableView::classify(rows, view::Viewport::new(20, 8))
        .with_column_width_mode(config.width);
    view.goto_user_row(config.start_position.row.max(1));
    if let Some(column) = config.start_position.column {
        view.goto_user_column(column.max(1));
    }

    let mut app = App {
        source,
        parse_options,
        view,
        popup: None,
        filter_prompt: None,
        search_query: String::new(),
        keys: command::KeyInterpreter::default(),
    };
    let mut terminal = ui::terminal::TerminalSession::enter()?;

    loop {
        terminal.terminal_mut().draw(|frame| {
            let area = frame.area();
            ui::render_table(&mut app.view, area, frame.buffer_mut());
            match app.popup {
                Some(ui::Popup::Help) => ui::render_help_popup(
                    &command::default_key_bindings(),
                    help_popup_area(area),
                    frame.buffer_mut(),
                ),
                Some(ui::Popup::Cell) => {
                    if let Some(cell) = current_cell(&app.view) {
                        ui::render_cell_popup(cell, "Cell", popup_area(area), frame.buffer_mut());
                    }
                }
                Some(ui::Popup::Info) => {
                    ui::render_info_popup(&app.info_text(), popup_area(area), frame.buffer_mut());
                }
                Some(ui::Popup::Search) => {
                    ui::render_search_prompt(
                        &app.search_query,
                        popup_area(area),
                        frame.buffer_mut(),
                    );
                }
                Some(ui::Popup::Filter) => {
                    if let Some(prompt) = &app.filter_prompt {
                        ui::render_filter_prompt(
                            &FilterPromptView::from(prompt),
                            popup_area(area),
                            frame.buffer_mut(),
                        );
                    }
                }
                None => {}
            }
        })?;

        if let Event::Key(event) = read()? {
            if app.handle_key(event)? {
                break;
            }
        }
    }
    Ok(())
}

struct App {
    source: ingest::source::InputSource,
    parse_options: ingest::ParseOptions,
    view: view::TableView,
    popup: Option<ui::Popup>,
    filter_prompt: Option<FilterPrompt>,
    search_query: String,
    keys: command::KeyInterpreter,
}

impl App {
    fn handle_key(&mut self, event: KeyEvent) -> anyhow::Result<bool> {
        if self.popup == Some(ui::Popup::Search) {
            self.handle_search_key(event);
            return Ok(false);
        }
        if self.popup == Some(ui::Popup::Filter) {
            self.handle_filter_key(event);
            return Ok(false);
        }

        if self.popup.is_some() {
            if closes_popup(event) {
                self.popup = None;
            }
            return Ok(false);
        }

        let Some(action) = self.key_action(event) else {
            return Ok(false);
        };

        self.apply(action)?;
        Ok(action.command == command::Command::Quit)
    }

    fn key_action(&mut self, event: KeyEvent) -> Option<command::KeyAction> {
        if event.modifiers == KeyModifiers::CONTROL {
            return command::lookup_key_event(event).map(|command| command::KeyAction {
                command,
                count: None,
            });
        }

        match event.code {
            KeyCode::Char(ch) => self.keys.handle_char(ch),
            _ => command::lookup_key_event(event).map(|command| command::KeyAction {
                command,
                count: None,
            }),
        }
    }

    fn apply(&mut self, action: command::KeyAction) -> anyhow::Result<()> {
        use command::Command;
        use ops::search::SearchDirection;
        use ops::skip::{Axis, Direction};
        use ops::sort::{SortDirection, SortMode};

        let count = action.count.unwrap_or(1);
        match action.command {
            Command::Quit => {}
            Command::Reload => self.reload()?,
            Command::MoveUp => self.view.move_by(-(count as isize), 0),
            Command::MoveDown => self.view.move_by(count as isize, 0),
            Command::MoveLeft => self.view.move_by(0, -(count as isize)),
            Command::MoveRight => self.view.move_by(0, count as isize),
            Command::Help => self.popup = Some(ui::Popup::Help),
            Command::PageUp => self.view.page_by(-1, 0, count),
            Command::PageDown => self.view.page_by(1, 0, count),
            Command::PageLeft => self.view.page_by(0, -1, count),
            Command::PageRight => self.view.page_by(0, 1, count),
            Command::LineHome => self.view.goto(self.view.cursor().row, 0),
            Command::LineEnd => {
                self.view.goto(
                    self.view.cursor().row,
                    self.view.column_count().saturating_sub(1),
                );
            }
            Command::GotoTop => self.view.goto_top(),
            Command::GotoRow => {
                if let Some(row) = action.count {
                    self.view.goto_user_row(row);
                } else {
                    self.view.goto_bottom();
                }
            }
            Command::GotoColumn => {
                if let Some(column) = action.count {
                    self.view.goto_user_column(column);
                } else {
                    self.view.goto(self.view.cursor().row, 0);
                }
            }
            Command::Mark => self.view.set_mark(),
            Command::GotoMark => self.view.goto_mark(),
            Command::ShowCell => {
                if current_cell(&self.view).is_some_and(|cell| !cell.is_empty()) {
                    self.popup = Some(ui::Popup::Cell);
                }
            }
            Command::Search => {
                self.search_query.clear();
                self.popup = Some(ui::Popup::Search);
            }
            Command::FilterIn => self.open_filter_prompt(FilterMode::In),
            Command::FilterOut => self.open_filter_prompt(FilterMode::Out),
            Command::NextSearchResult => self.search(SearchDirection::Forward),
            Command::PreviousSearchResult => self.search(SearchDirection::Reverse),
            Command::ToggleHeader => self.view.toggle_header(),
            Command::GapDown => self.view.adjust_column_gap(-(count as isize)),
            Command::GapUp => self.view.adjust_column_gap(count as isize),
            Command::AllColumnsNarrower => self.view.adjust_all_column_widths(-(count as isize)),
            Command::AllColumnsWider => self.view.adjust_all_column_widths(count as isize),
            Command::CurrentColumnNarrower => {
                self.view.adjust_current_column_width(-(count as isize));
            }
            Command::CurrentColumnWider => self.view.adjust_current_column_width(count as isize),
            Command::SortNaturalAsc => self
                .view
                .sort_current_column(SortMode::Natural, SortDirection::Ascending),
            Command::SortNaturalDesc => self
                .view
                .sort_current_column(SortMode::Natural, SortDirection::Descending),
            Command::SortNumericAsc => self
                .view
                .sort_current_column(SortMode::Numeric, SortDirection::Ascending),
            Command::SortNumericDesc => self
                .view
                .sort_current_column(SortMode::Numeric, SortDirection::Descending),
            Command::SortLexicalAsc => self
                .view
                .sort_current_column(SortMode::Lexical, SortDirection::Ascending),
            Command::SortLexicalDesc => self
                .view
                .sort_current_column(SortMode::Lexical, SortDirection::Descending),
            Command::YankCell => {
                let rows = self.view.visible_rows_vec();
                let _ = ops::clipboard::yank_cell(&rows, self.view.cursor());
            }
            Command::ToggleColumnWidthMode => {
                if let Some(width) = action.count {
                    self.view.set_all_column_widths(width);
                } else {
                    self.view.toggle_variable_column_width_mode();
                }
            }
            Command::SetCurrentColumnWidth => {
                if let Some(width) = action.count {
                    self.view.set_current_column_width(width);
                } else {
                    self.view.maximize_current_column_width();
                }
            }
            Command::SkipRowChangeForward => {
                let rows = self.view.visible_rows_vec();
                let position = ops::skip::skip_to_change(
                    &rows,
                    self.view.cursor(),
                    Axis::Row,
                    Direction::Forward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::SkipRowChangeBackward => {
                let rows = self.view.visible_rows_vec();
                let position = ops::skip::skip_to_change(
                    &rows,
                    self.view.cursor(),
                    Axis::Row,
                    Direction::Backward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::SkipColumnChangeForward => {
                let rows = self.view.visible_rows_vec();
                let position = ops::skip::skip_to_change(
                    &rows,
                    self.view.cursor(),
                    Axis::Column,
                    Direction::Forward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::SkipColumnChangeBackward => {
                let rows = self.view.visible_rows_vec();
                let position = ops::skip::skip_to_change(
                    &rows,
                    self.view.cursor(),
                    Axis::Column,
                    Direction::Backward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::ShowInfo => self.popup = Some(ui::Popup::Info),
            Command::Redraw => {}
        }
        Ok(())
    }

    fn handle_search_key(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Esc | KeyCode::Enter => self.popup = None,
            KeyCode::Char('\n' | '\r') => self.popup = None,
            KeyCode::Backspace => {
                self.search_query.pop();
                self.search_current_or_next();
            }
            KeyCode::Char(ch)
                if event.modifiers.is_empty() || event.modifiers == KeyModifiers::SHIFT =>
            {
                self.search_query.push(ch);
                self.search_current_or_next();
            }
            _ => {}
        }
    }

    fn open_filter_prompt(&mut self, mode: FilterMode) {
        let column = self.view.cursor().column;
        self.filter_prompt = Some(FilterPrompt::new(&self.view, mode, column));
        self.popup = Some(ui::Popup::Filter);
    }

    fn handle_filter_key(&mut self, event: KeyEvent) {
        let Some(prompt) = &mut self.filter_prompt else {
            self.popup = None;
            return;
        };
        match event.code {
            KeyCode::Esc => {
                self.filter_prompt = None;
                self.popup = None;
            }
            KeyCode::Enter | KeyCode::Char('\n' | '\r') => self.submit_filter_prompt(),
            KeyCode::Tab => prompt.cycle_kind(),
            KeyCode::Backspace => {
                prompt.input.pop();
                prompt.error = None;
            }
            KeyCode::Char(ch)
                if event.modifiers.is_empty() || event.modifiers == KeyModifiers::SHIFT =>
            {
                prompt.input.push(ch);
                prompt.error = None;
            }
            _ => {}
        }
    }

    fn submit_filter_prompt(&mut self) {
        let Some(prompt) = &self.filter_prompt else {
            self.popup = None;
            return;
        };
        if prompt.input.trim().is_empty() {
            self.view.clear_filters_for_column(prompt.column);
            self.filter_prompt = None;
            self.popup = None;
            return;
        }
        let result = self.view.apply_filter(
            prompt.column,
            prompt.mode,
            prompt.selected_kind,
            prompt.input.clone(),
        );
        match result {
            Ok(()) => {
                self.filter_prompt = None;
                self.popup = None;
            }
            Err(err) => {
                if let Some(prompt) = &mut self.filter_prompt {
                    prompt.error = Some(err.to_string());
                }
            }
        }
    }

    fn search_current_or_next(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        if current_cell(&self.view).is_some_and(|cell| {
            cell.to_lowercase()
                .contains(&self.search_query.to_lowercase())
        }) {
            return;
        }
        self.search(ops::search::SearchDirection::Forward);
    }

    fn search(&mut self, direction: ops::search::SearchDirection) {
        let rows = self.view.visible_rows_vec();
        if let Some(position) =
            ops::search::find_match(&rows, self.view.cursor(), &self.search_query, direction)
        {
            self.view.goto(position.row, position.column);
        }
    }

    fn reload(&mut self) -> anyhow::Result<()> {
        if self.source == ingest::source::InputSource::Stdin {
            return Ok(());
        }

        let cursor = self.view.cursor();
        let viewport = self.view.viewport();
        let rows = read_rows(&self.source, &self.parse_options)?;
        let mut reloaded = view::TableView::classify(rows, viewport);
        reloaded.restore_view_settings_from(&self.view);
        reloaded.goto(cursor.row, cursor.column);
        self.view = reloaded;
        Ok(())
    }

    fn info_text(&self) -> String {
        format!(
            "Rows: {}\nColumns: {}\nPosition: {},{}\nWidth mode: {:?}\nColumn gap: {}\nMark: {}",
            self.view.row_count(),
            self.view.column_count(),
            self.view.cursor().row + 1,
            self.view.cursor().column + 1,
            self.view.column_width_mode(),
            self.view.column_gap(),
            self.view
                .mark()
                .map(|position| format!("{},{}", position.row + 1, position.column + 1))
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilterPrompt {
    mode: FilterMode,
    column: usize,
    selected_kind: FilterKind,
    enabled_kinds: Vec<FilterKind>,
    input: String,
    error: Option<String>,
}

impl FilterPrompt {
    fn new(view: &view::TableView, mode: FilterMode, column: usize) -> Self {
        let enabled_kinds = FilterKind::all()
            .into_iter()
            .filter(|kind| view.filter_kind_enabled(column, *kind))
            .collect::<Vec<_>>();
        let selected_kind = view.default_filter_kind(column);
        Self {
            mode,
            column,
            selected_kind,
            enabled_kinds,
            input: String::new(),
            error: None,
        }
    }

    fn cycle_kind(&mut self) {
        if self.enabled_kinds.is_empty() {
            return;
        }
        let current = self
            .enabled_kinds
            .iter()
            .position(|kind| *kind == self.selected_kind)
            .unwrap_or(0);
        self.selected_kind = self.enabled_kinds[(current + 1) % self.enabled_kinds.len()];
        self.error = None;
    }
}

pub(crate) struct FilterPromptView<'a> {
    pub mode: FilterMode,
    pub column: usize,
    pub selected_kind: FilterKind,
    pub enabled_kinds: &'a [FilterKind],
    pub input: &'a str,
    pub error: Option<&'a str>,
}

impl<'a> From<&'a FilterPrompt> for FilterPromptView<'a> {
    fn from(prompt: &'a FilterPrompt) -> Self {
        Self {
            mode: prompt.mode,
            column: prompt.column,
            selected_kind: prompt.selected_kind,
            enabled_kinds: &prompt.enabled_kinds,
            input: &prompt.input,
            error: prompt.error.as_deref(),
        }
    }
}

fn read_rows(
    source: &ingest::source::InputSource,
    parse_options: &ingest::ParseOptions,
) -> anyhow::Result<Vec<Vec<String>>> {
    let bytes = ingest::source::read_source(source)?;
    Ok(ingest::parse_rows(&bytes, parse_options)?)
}

fn closes_popup(event: KeyEvent) -> bool {
    matches!(event.code, KeyCode::Esc | KeyCode::Enter)
        || matches!(
            command::lookup_key_event(event),
            Some(command::Command::Quit | command::Command::Help)
        )
}

fn current_cell(view: &view::TableView) -> Option<&str> {
    view.current_cell()
}

fn popup_area(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let width = (area.width.saturating_mul(3) / 4).max(20).min(area.width);
    let height = (area.height.saturating_mul(3) / 4).max(5).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    ratatui::layout::Rect::new(x, y, width, height)
}

fn help_popup_area(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    if area.width <= 4 || area.height <= 4 {
        return area;
    }
    ratatui::layout::Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use std::io::{Seek, Write};

    fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect()
    }

    fn app_with_rows(rows: Vec<Vec<String>>) -> App {
        App {
            source: ingest::source::InputSource::Stdin,
            parse_options: ingest::ParseOptions::default(),
            view: view::TableView::classify(rows, view::Viewport::new(10, 4)),
            popup: None,
            filter_prompt: None,
            search_query: String::new(),
            keys: command::KeyInterpreter::default(),
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn filter_prompt_defaults_from_column_type_and_tabs_enabled_kinds() {
        let mut app = app_with_rows(rows(&[&["Name", "Size"], &["alpha", "1gb"]]));

        app.handle_key(key(KeyCode::Char('f'))).expect("filter in");
        let prompt = app.filter_prompt.as_ref().expect("text prompt");
        assert_eq!(prompt.selected_kind, FilterKind::Text);
        assert!(!prompt.enabled_kinds.contains(&FilterKind::Numeric));

        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        app.view.goto(0, 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT))
            .expect("filter out");
        let prompt = app.filter_prompt.as_ref().expect("numeric prompt");
        assert_eq!(prompt.mode, FilterMode::Out);
        assert_eq!(prompt.selected_kind, FilterKind::Numeric);
        assert!(prompt.enabled_kinds.contains(&FilterKind::Text));
        assert!(prompt.enabled_kinds.contains(&FilterKind::Regex));
        assert!(prompt.enabled_kinds.contains(&FilterKind::Numeric));

        app.handle_key(key(KeyCode::Tab)).expect("cycle");
        app.handle_key(key(KeyCode::Char('g'))).expect("type");
        let prompt = app.filter_prompt.as_ref().expect("cycled prompt");
        assert_eq!(prompt.selected_kind, FilterKind::Text);
        assert_eq!(prompt.input, "g");
    }

    #[test]
    fn filter_prompt_applies_cancels_and_clears_filters() {
        let mut app = app_with_rows(rows(&[&["Name"], &["alpha"], &["beta"]]));

        app.handle_key(key(KeyCode::Char('f'))).expect("filter in");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        assert_eq!(app.view.row_count(), 2);

        app.handle_key(key(KeyCode::Char('f'))).expect("filter in");
        for ch in "alp".chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("submit");
        assert_eq!(app.view.visible_rows_vec(), rows(&[&["alpha"]]));

        app.handle_key(key(KeyCode::Char('f'))).expect("filter in");
        app.handle_key(key(KeyCode::Enter)).expect("clear");
        assert_eq!(app.view.row_count(), 2);
    }

    #[test]
    fn reload_reapplies_active_filters_and_clamps_cursor() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(file, "Name").expect("write header");
        writeln!(file, "alpha").expect("write row");
        writeln!(file, "beta").expect("write row");

        let mut app = App {
            source: ingest::source::InputSource::Path(file.path().to_path_buf()),
            parse_options: ingest::ParseOptions::default(),
            view: view::TableView::classify(
                rows(&[&["Name"], &["alpha"], &["beta"]]),
                view::Viewport::new(10, 4),
            ),
            popup: None,
            filter_prompt: None,
            search_query: String::new(),
            keys: command::KeyInterpreter::default(),
        };
        app.view
            .apply_filter(0, FilterMode::In, FilterKind::Text, "alp".to_owned())
            .expect("apply filter");
        app.view.goto(10, 0);

        file.as_file_mut().set_len(0).expect("truncate");
        file.rewind().expect("rewind");
        writeln!(file, "Name").expect("write header");
        writeln!(file, "alpha").expect("write row");
        writeln!(file, "gamma").expect("write row");
        file.flush().expect("flush");

        app.reload().expect("reload");

        assert_eq!(app.view.visible_rows_vec(), rows(&[&["alpha"]]));
        assert_eq!(app.view.cursor(), view::Position { row: 0, column: 0 });
    }
}
