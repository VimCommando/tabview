pub mod cli;
pub mod command;
pub mod compat;
pub mod ingest;
pub mod ops;
pub mod table;
pub mod ui;
pub mod view;

pub fn run(args: cli::Args) -> anyhow::Result<()> {
    use crossterm::event::{read, Event, KeyCode};

    let config = cli::Config::from_args(args)?;
    let source = ingest::source::InputSource::from_cli_value(&config.filename.to_string_lossy());
    let bytes = ingest::source::read_source(&source)?;
    let parse_options = ingest::ParseOptions {
        encoding: config.encoding,
        delimiter: config.delimiter,
        quoting: config.quoting,
        quote_char: config.quote_char as u8,
    };
    let rows = ingest::parse_rows(&bytes, &parse_options)?;
    let mut view = view::TableView::classify(rows, view::Viewport::new(20, 8));
    let mut terminal = ui::terminal::TerminalSession::enter()?;
    let mut popup = None;

    loop {
        terminal.terminal_mut().draw(|frame| {
            let area = frame.area();
            ui::render_table(&mut view, area, frame.buffer_mut());
            match popup {
                Some(ui::Popup::Help) => ui::render_help_popup(
                    &command::default_key_bindings(),
                    popup_area(area),
                    frame.buffer_mut(),
                ),
                Some(ui::Popup::Cell) => {
                    if let Some(cell) = current_cell(&view) {
                        ui::render_cell_popup(cell, "Cell", popup_area(area), frame.buffer_mut());
                    }
                }
                Some(ui::Popup::Info) => {
                    let info = format!(
                        "Rows: {}\nColumns: {}\nPosition: {},{}",
                        view.rows().len(),
                        view.column_count(),
                        view.cursor().row + 1,
                        view.cursor().column + 1
                    );
                    ui::render_info_popup(&info, popup_area(area), frame.buffer_mut());
                }
                Some(ui::Popup::Search) | None => {}
            }
        })?;

        if let Event::Key(event) = read()? {
            if popup.is_some() {
                if matches!(event.code, KeyCode::Esc | KeyCode::Enter) {
                    popup = None;
                    continue;
                }
                if matches!(
                    command::lookup_key_event(event),
                    Some(command::Command::Quit)
                ) {
                    popup = None;
                    continue;
                }
            }

            match command::lookup_key_event(event) {
                Some(command::Command::Quit) => break,
                Some(command::Command::MoveUp) => view.move_by(-1, 0),
                Some(command::Command::MoveDown) => view.move_by(1, 0),
                Some(command::Command::MoveLeft) => view.move_by(0, -1),
                Some(command::Command::MoveRight) => view.move_by(0, 1),
                Some(command::Command::LineHome) => view.goto(view.cursor().row, 0),
                Some(command::Command::LineEnd) => {
                    view.goto(view.cursor().row, view.column_count().saturating_sub(1));
                }
                Some(command::Command::ToggleHeader) => view.toggle_header(),
                Some(command::Command::Help) => popup = Some(ui::Popup::Help),
                Some(command::Command::ShowCell)
                    if current_cell(&view).is_some_and(|cell| !cell.is_empty()) =>
                {
                    popup = Some(ui::Popup::Cell);
                }
                Some(command::Command::ShowInfo) => popup = Some(ui::Popup::Info),
                Some(command::Command::SortLexicalAsc) => view.sort_current_column(
                    ops::sort::SortMode::Lexical,
                    ops::sort::SortDirection::Ascending,
                ),
                Some(command::Command::SortLexicalDesc) => view.sort_current_column(
                    ops::sort::SortMode::Lexical,
                    ops::sort::SortDirection::Descending,
                ),
                Some(command::Command::SortNaturalAsc) => view.sort_current_column(
                    ops::sort::SortMode::Natural,
                    ops::sort::SortDirection::Ascending,
                ),
                Some(command::Command::SortNaturalDesc) => view.sort_current_column(
                    ops::sort::SortMode::Natural,
                    ops::sort::SortDirection::Descending,
                ),
                Some(command::Command::SortNumericAsc) => view.sort_current_column(
                    ops::sort::SortMode::Numeric,
                    ops::sort::SortDirection::Ascending,
                ),
                Some(command::Command::SortNumericDesc) => view.sort_current_column(
                    ops::sort::SortMode::Numeric,
                    ops::sort::SortDirection::Descending,
                ),
                _ => {}
            }
        }
    }
    Ok(())
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
