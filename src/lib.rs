pub mod cli;
pub mod command;
pub mod compat;
pub mod ingest;
pub mod ops;
pub mod table;
pub mod ui;
pub mod view;

pub fn run(args: cli::Args) -> anyhow::Result<()> {
    use crossterm::event::{read, Event};

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

    loop {
        terminal.terminal_mut().draw(|frame| {
            ui::render_table(&view, frame.area(), frame.buffer_mut());
        })?;

        if let Event::Key(event) = read()? {
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
                _ => {}
            }
        }
    }
    Ok(())
}
