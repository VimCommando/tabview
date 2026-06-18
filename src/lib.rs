pub mod cli;
pub mod command;
pub mod ingest;
pub mod ops;
pub mod table;
pub mod ui;
pub mod view;

use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};

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
    search_query: String,
    keys: command::KeyInterpreter,
}

impl App {
    fn handle_key(&mut self, event: KeyEvent) -> anyhow::Result<bool> {
        if self.popup == Some(ui::Popup::Search) {
            self.handle_search_key(event);
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
                let _ = ops::clipboard::yank_cell(self.view.rows(), self.view.cursor());
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
                let position = ops::skip::skip_to_change(
                    self.view.rows(),
                    self.view.cursor(),
                    Axis::Row,
                    Direction::Forward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::SkipRowChangeBackward => {
                let position = ops::skip::skip_to_change(
                    self.view.rows(),
                    self.view.cursor(),
                    Axis::Row,
                    Direction::Backward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::SkipColumnChangeForward => {
                let position = ops::skip::skip_to_change(
                    self.view.rows(),
                    self.view.cursor(),
                    Axis::Column,
                    Direction::Forward,
                    count,
                );
                self.view.goto(position.row, position.column);
            }
            Command::SkipColumnChangeBackward => {
                let position = ops::skip::skip_to_change(
                    self.view.rows(),
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
        if let Some(position) = ops::search::find_match(
            self.view.rows(),
            self.view.cursor(),
            &self.search_query,
            direction,
        ) {
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
    view.rows()
        .get(view.cursor().row)
        .and_then(|row| row.get(view.cursor().column))
        .map(String::as_str)
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
