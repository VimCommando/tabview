pub mod cli;
pub mod command;
pub mod ingest;
pub mod ops;
#[cfg(feature = "saved-views")]
pub mod saved_views;
pub mod table;
pub mod theme;
pub mod ui;
pub mod view;

use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use ops::filter::{FilterKind, FilterMode};
#[cfg(feature = "saved-views")]
use std::path::{Path, PathBuf};

pub fn run(args: cli::Args) -> anyhow::Result<()> {
    let config = cli::Config::from_args(args)?;
    let theme_load = theme::load_active_theme(None)?;
    let source = ingest::source::InputSource::from_cli_value(&config.filename.to_string_lossy());
    let parse_options = ingest::ParseOptions {
        encoding: config.encoding.clone(),
        delimiter: config.delimiter,
        quoting: config.quoting,
        quote_char: config.quote_char,
    };
    let rows = read_rows(&source, &parse_options)?;
    let mut view = view::TableView::classify(rows, view::Viewport::new(20, 8))
        .with_column_width_mode(config.width);
    #[cfg(feature = "saved-views")]
    let saved_view = apply_saved_view(&config, &mut view)?;
    #[cfg(not(feature = "saved-views"))]
    let saved_view_message = None::<String>;
    #[cfg(feature = "saved-views")]
    let saved_view_message = saved_view
        .as_ref()
        .and_then(|saved_view| saved_view.messages.first().cloned());
    let message = saved_view_message.or_else(|| {
        theme_load
            .warnings
            .first()
            .map(|warning| format!("theme warning: {}: {}", warning.field, warning.message))
    });
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
        column_info: None,
        search_query: String::new(),
        keys: command::KeyInterpreter::default(),
        message,
        theme: theme_load.theme,
        #[cfg(feature = "saved-views")]
        saved_view,
        #[cfg(feature = "saved-views")]
        view_modal: None,
    };
    let mut terminal = ui::terminal::TerminalSession::enter()?;

    loop {
        terminal.terminal_mut().draw(|frame| {
            let area = frame.area();
            let table_area = table_area(area);
            ui::render_table_with_theme(
                &mut app.view,
                table_area,
                frame.buffer_mut(),
                &app.theme,
                Some(&app.search_query),
            );
            ui::render_footer_with_theme(
                app.message.as_deref(),
                area,
                frame.buffer_mut(),
                &app.theme,
            );
            match app.popup {
                Some(ui::Popup::Help) => ui::render_help_popup_with_theme(
                    &command::default_key_bindings(),
                    help_popup_area(area),
                    frame.buffer_mut(),
                    &app.theme,
                ),
                Some(ui::Popup::Cell) => {
                    if let Some(cell) = current_cell(&app.view) {
                        ui::render_cell_popup_with_theme(
                            &cell,
                            "Cell",
                            popup_area(area),
                            frame.buffer_mut(),
                            &app.theme,
                        );
                    }
                }
                Some(ui::Popup::Info) => {
                    ui::render_info_popup_with_theme(
                        &app.info_text(),
                        popup_area(area),
                        frame.buffer_mut(),
                        &app.theme,
                    );
                }
                Some(ui::Popup::Search) => {
                    ui::render_search_prompt_with_theme(
                        &app.search_query,
                        popup_area(area),
                        frame.buffer_mut(),
                        &app.theme,
                    );
                }
                Some(ui::Popup::Filter) => {
                    if let Some(prompt) = &app.filter_prompt {
                        ui::render_filter_prompt_with_theme(
                            &FilterPromptView::from(prompt),
                            popup_area(area),
                            frame.buffer_mut(),
                            &app.theme,
                        );
                    }
                }
                Some(ui::Popup::ColumnInfo) => {
                    if let Some(modal) = &app.column_info {
                        ui::render_column_info_popup_with_theme(
                            &modal.popup(),
                            popup_area(area),
                            frame.buffer_mut(),
                            &app.theme,
                        );
                    }
                }
                #[cfg(feature = "saved-views")]
                Some(ui::Popup::SavedView) => {
                    if let Some(modal) = &app.view_modal {
                        ui::render_saved_view_popup_with_theme(
                            &modal.filename,
                            &modal.yaml,
                            modal.scroll,
                            modal.confirming_overwrite,
                            popup_area(area),
                            frame.buffer_mut(),
                            &app.theme,
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
    column_info: Option<ColumnInfoModal>,
    search_query: String,
    keys: command::KeyInterpreter,
    message: Option<String>,
    theme: theme::ResolvedTheme,
    #[cfg(feature = "saved-views")]
    saved_view: Option<SavedViewRuntime>,
    #[cfg(feature = "saved-views")]
    view_modal: Option<ViewModal>,
}

#[cfg(feature = "saved-views")]
#[derive(Debug, Clone)]
struct SavedViewRuntime {
    source_path: Option<PathBuf>,
    target_path: Option<PathBuf>,
    view_name: String,
    explicit_locale: Option<String>,
    messages: Vec<String>,
}

#[cfg(feature = "saved-views")]
#[derive(Debug, Clone)]
struct ViewModal {
    filename: String,
    yaml: String,
    scroll: usize,
    confirming_overwrite: bool,
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
        if self.popup == Some(ui::Popup::ColumnInfo) {
            self.handle_column_info_key(event);
            return Ok(false);
        }
        #[cfg(feature = "saved-views")]
        if self.popup == Some(ui::Popup::SavedView) {
            self.handle_saved_view_key(event);
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
            Command::ColumnInfo => self.open_column_info_modal(),
            #[cfg(feature = "saved-views")]
            Command::SavedView => self.open_saved_view_modal(),
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
            Command::YankRawCell => {
                let rows = self.view.visible_raw_rows_vec();
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
            Command::ColumnHideLeft => self.view.hide_columns_left(count),
            Command::ColumnHideRight => self.view.hide_columns_right(count),
            Command::ColumnHideCurrent => self.view.hide_current_column(),
            Command::ColumnShowLeft => self.view.show_hidden_left(count),
            Command::ColumnShowRight => self.view.show_hidden_right(count),
            Command::ColumnSortAsc => {
                let mode = if self.view.is_numeric_column(self.view.cursor().column) {
                    SortMode::Numeric
                } else {
                    SortMode::Lexical
                };
                self.view
                    .sort_current_column(mode, SortDirection::Ascending);
            }
            Command::ColumnSortDesc => {
                let mode = if self.view.is_numeric_column(self.view.cursor().column) {
                    SortMode::Numeric
                } else {
                    SortMode::Lexical
                };
                self.view
                    .sort_current_column(mode, SortDirection::Descending);
            }
            Command::ColumnSortClear => self.view.clear_current_column_sort(),
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

    fn open_column_info_modal(&mut self) {
        if let Some(info) = self.view.current_column_info() {
            self.column_info = Some(ColumnInfoModal::from_info(info));
            self.popup = Some(ui::Popup::ColumnInfo);
        }
    }

    fn handle_column_info_key(&mut self, event: KeyEvent) {
        let Some(modal) = &mut self.column_info else {
            self.popup = None;
            return;
        };
        match event.code {
            KeyCode::Esc => {
                self.column_info = None;
                self.popup = None;
            }
            KeyCode::Enter | KeyCode::Char('\n' | '\r') => {
                let update = modal.to_update();
                self.view.apply_current_column_info(update);
                self.column_info = None;
                self.popup = None;
                self.message = Some("column view updated".to_owned());
            }
            KeyCode::Tab => modal.next_group(),
            KeyCode::BackTab => modal.previous_group(),
            KeyCode::Up | KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('h') => {
                modal.previous_option();
            }
            KeyCode::Down | KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('l') => {
                modal.next_option();
            }
            _ => {}
        }
    }

    fn search_current_or_next(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        if self.view.current_cell_matches(&self.search_query) {
            return;
        }
        self.search(ops::search::SearchDirection::Forward);
    }

    fn search(&mut self, direction: ops::search::SearchDirection) {
        let rows = self.view.search_rows_vec();
        if let Some(position) =
            ops::search::find_match(&rows, self.view.cursor(), &self.search_query, direction)
        {
            self.view.goto(position.row, position.column);
        }
    }

    #[cfg(feature = "saved-views")]
    fn open_saved_view_modal(&mut self) {
        let Some(saved_view) = &self.saved_view else {
            self.message = Some("saved views are disabled".to_owned());
            return;
        };
        let input_filename = self.input_filename();
        let yaml = self.view.to_saved_view_yaml(
            &saved_view.view_name,
            &input_filename,
            saved_view.explicit_locale.as_deref(),
        );
        let filename = saved_view
            .target_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<no saved view path>".to_owned());
        self.view_modal = Some(ViewModal {
            filename,
            yaml,
            scroll: 0,
            confirming_overwrite: false,
        });
        self.popup = Some(ui::Popup::SavedView);
    }

    #[cfg(feature = "saved-views")]
    fn handle_saved_view_key(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Esc => {
                self.view_modal = None;
                self.popup = None;
            }
            KeyCode::Char('s') => self.save_view_modal(false),
            KeyCode::Char('y')
                if self
                    .view_modal
                    .as_ref()
                    .is_some_and(|modal| modal.confirming_overwrite) =>
            {
                self.save_view_modal(true);
            }
            KeyCode::Char('n') => {
                if let Some(modal) = &mut self.view_modal {
                    modal.confirming_overwrite = false;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(modal) = &mut self.view_modal {
                    modal.scroll = modal.scroll.saturating_add(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(modal) = &mut self.view_modal {
                    modal.scroll = modal.scroll.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    #[cfg(feature = "saved-views")]
    fn save_view_modal(&mut self, confirmed_overwrite: bool) {
        let Some(saved_view) = &mut self.saved_view else {
            self.message = Some("saved views are disabled".to_owned());
            return;
        };
        let Some(target_path) = saved_view.target_path.clone() else {
            self.message = Some("saved view path is unavailable".to_owned());
            return;
        };
        let Some(modal) = &mut self.view_modal else {
            return;
        };
        if target_path.exists() && !confirmed_overwrite {
            modal.confirming_overwrite = true;
            return;
        }

        match write_saved_view_atomic(&target_path, &modal.yaml) {
            Ok(()) => {
                saved_view.source_path = Some(target_path.clone());
                saved_view.target_path = Some(target_path.clone());
                modal.confirming_overwrite = false;
                self.message = Some(format!("saved view {}", target_path.display()));
            }
            Err(err) => {
                modal.confirming_overwrite = false;
                self.message = Some(format!("failed to save view: {err}"));
                eprintln!("failed to save view {}: {err}", target_path.display());
            }
        }
    }

    #[cfg(feature = "saved-views")]
    fn input_filename(&self) -> String {
        match &self.source {
            ingest::source::InputSource::Path(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("input")
                .to_owned(),
            ingest::source::InputSource::Stdin => "-".to_owned(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnInfoGroup {
    Visibility,
    Align,
    Type,
    Format,
    Sort,
    Filters,
}

impl ColumnInfoGroup {
    const VISUAL: [Self; 6] = [
        Self::Visibility,
        Self::Format,
        Self::Align,
        Self::Sort,
        Self::Type,
        Self::Filters,
    ];
    const TAB_ORDER: [Self; 6] = [
        Self::Visibility,
        Self::Align,
        Self::Type,
        Self::Format,
        Self::Sort,
        Self::Filters,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Visibility => "Visibility",
            Self::Align => "Align",
            Self::Type => "Type",
            Self::Format => "Format",
            Self::Sort => "Sort",
            Self::Filters => "Filters",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnInfoModal {
    column_name: String,
    visible_column: usize,
    source_column: usize,
    filter_details: Vec<String>,
    filter_indicator: Option<&'static str>,
    active_group: usize,
    visibility: usize,
    alignment: usize,
    column_type: usize,
    format: usize,
    sort: usize,
    filters: usize,
}

impl ColumnInfoModal {
    fn from_info(info: view::ColumnInfo) -> Self {
        let filter_indicator = filter_indicator(&info.filters);
        Self {
            column_name: info.name,
            visible_column: info.visible_column,
            source_column: info.source_column,
            filter_details: if info.filters.is_empty() {
                vec!["None".to_owned()]
            } else {
                info.filters
                    .into_iter()
                    .map(|filter| format!("{} {} {}", filter.mode, filter.kind, filter.input))
                    .collect()
            },
            filter_indicator,
            active_group: 0,
            visibility: usize::from(!info.visible),
            alignment: match info.alignment {
                None => 0,
                Some(view::ColumnAlignment::Left) => 1,
                Some(view::ColumnAlignment::Right) => 2,
            },
            column_type: column_type_index(info.column_type),
            format: column_format_index(info.format),
            sort: match info.sort {
                view::ColumnSortChoice::None => 0,
                view::ColumnSortChoice::Ascending => 1,
                view::ColumnSortChoice::Descending => 2,
            },
            filters: 0,
        }
    }

    fn popup(&self) -> ui::ColumnInfoPopup {
        ui::ColumnInfoPopup {
            title: "Column Info".to_owned(),
            summary: format!(
                "{}  visible:{} source:{}",
                self.column_name,
                self.visible_column + 1,
                self.source_column + 1
            ),
            sections: ColumnInfoGroup::VISUAL
                .into_iter()
                .map(|group| ui::ColumnInfoSection {
                    header: group.label().to_owned(),
                    active: group == self.active_group(),
                    options: self.section_options(group),
                    details: if group == ColumnInfoGroup::Filters {
                        self.filter_details.clone()
                    } else {
                        Vec::new()
                    },
                })
                .collect(),
        }
    }

    fn next_group(&mut self) {
        self.active_group = (self.active_group + 1) % ColumnInfoGroup::TAB_ORDER.len();
    }

    fn previous_group(&mut self) {
        self.active_group = self
            .active_group
            .checked_sub(1)
            .unwrap_or(ColumnInfoGroup::TAB_ORDER.len() - 1);
    }

    fn next_option(&mut self) {
        let group = self.active_group();
        let options = self.option_count(group);
        if options == 0 {
            return;
        }
        let mut index = self.selected_index(group);
        for _ in 0..options {
            index = (index + 1) % options;
            if self.option_enabled(group, index) {
                self.set_selected_index(group, index);
                break;
            }
        }
    }

    fn previous_option(&mut self) {
        let group = self.active_group();
        let options = self.option_count(group);
        if options == 0 {
            return;
        }
        let mut index = self.selected_index(group);
        for _ in 0..options {
            index = index.checked_sub(1).unwrap_or(options - 1);
            if self.option_enabled(group, index) {
                self.set_selected_index(group, index);
                break;
            }
        }
    }

    fn active_group(&self) -> ColumnInfoGroup {
        ColumnInfoGroup::TAB_ORDER[self.active_group]
    }

    fn selected_index(&self, group: ColumnInfoGroup) -> usize {
        match group {
            ColumnInfoGroup::Visibility => self.visibility,
            ColumnInfoGroup::Align => self.alignment,
            ColumnInfoGroup::Type => self.column_type,
            ColumnInfoGroup::Format => self.format,
            ColumnInfoGroup::Sort => self.sort,
            ColumnInfoGroup::Filters => self.filters,
        }
    }

    fn set_selected_index(&mut self, group: ColumnInfoGroup, index: usize) {
        match group {
            ColumnInfoGroup::Visibility => self.visibility = index,
            ColumnInfoGroup::Align => self.alignment = index,
            ColumnInfoGroup::Type => {
                self.column_type = index;
                if !self.option_enabled(ColumnInfoGroup::Format, self.format) {
                    self.format = 0;
                }
            }
            ColumnInfoGroup::Format => self.format = index,
            ColumnInfoGroup::Sort => self.sort = index,
            ColumnInfoGroup::Filters => self.filters = index,
        }
    }

    fn option_count(&self, group: ColumnInfoGroup) -> usize {
        match group {
            ColumnInfoGroup::Visibility => 2,
            ColumnInfoGroup::Align => 3,
            ColumnInfoGroup::Type => 7,
            ColumnInfoGroup::Format => 7,
            ColumnInfoGroup::Sort => 3,
            ColumnInfoGroup::Filters => usize::from(self.filter_indicator.is_some()) + 1,
        }
    }

    fn option_enabled(&self, group: ColumnInfoGroup, index: usize) -> bool {
        match group {
            ColumnInfoGroup::Format => format_valid_for_type(self.column_type, index),
            ColumnInfoGroup::Filters => self.filter_indicator.is_some(),
            _ => true,
        }
    }

    fn section_options(&self, group: ColumnInfoGroup) -> Vec<ui::ColumnInfoOption> {
        if group == ColumnInfoGroup::Filters && self.filter_indicator.is_none() {
            return Vec::new();
        }
        (0..self.option_count(group))
            .map(|idx| ui::ColumnInfoOption {
                label: self.option_label(group, idx),
                selected: idx == self.selected_index(group),
                enabled: self.option_enabled(group, idx),
            })
            .collect()
    }

    fn option_label(&self, group: ColumnInfoGroup, index: usize) -> String {
        match group {
            ColumnInfoGroup::Visibility => ["visible", "hidden"][index].to_owned(),
            ColumnInfoGroup::Align => ["auto", "left", "right"][index].to_owned(),
            ColumnInfoGroup::Type => [
                "text", "date", "ip", "float", "integer", "semver", "boolean",
            ][index]
                .to_owned(),
            ColumnInfoGroup::Format => [
                "plain",
                "locale",
                "uppercase",
                "lowercase",
                "char",
                "bit",
                "word",
            ][index]
                .to_owned(),
            ColumnInfoGroup::Sort => ["none", "▲ ascending", "▼ descending"][index].to_owned(),
            ColumnInfoGroup::Filters => {
                let indicator = self.filter_indicator.unwrap_or(" ");
                match index {
                    0 => format!("keep {indicator}"),
                    _ => format!("clear {indicator}"),
                }
            }
        }
    }

    fn to_update(&self) -> view::ColumnInfoUpdate {
        view::ColumnInfoUpdate {
            visible: self.visibility == 0,
            alignment: match self.alignment {
                1 => Some(view::ColumnAlignment::Left),
                2 => Some(view::ColumnAlignment::Right),
                _ => None,
            },
            column_type: column_type_choice(self.column_type),
            format: column_format_choice(self.format),
            sort: match self.sort {
                1 => view::ColumnSortChoice::Ascending,
                2 => view::ColumnSortChoice::Descending,
                _ => view::ColumnSortChoice::None,
            },
            clear_filters: self.filters == 1,
        }
    }
}

fn filter_indicator(filters: &[view::ColumnFilterSummary]) -> Option<&'static str> {
    let first = filters.first()?;
    if filters.len() > 1 {
        return Some("±");
    }
    Some(match first.mode {
        "in" => "+",
        "out" => "-",
        _ => "±",
    })
}

fn format_valid_for_type(column_type: usize, format: usize) -> bool {
    match format {
        0 => true,
        1 => matches!(column_type, 3 | 4),
        2 | 3 => matches!(column_type, 0 | 1 | 2 | 5),
        4..=6 => column_type == 6,
        _ => false,
    }
}

fn column_type_index(choice: view::ColumnTypeChoice) -> usize {
    match choice {
        view::ColumnTypeChoice::Text => 0,
        view::ColumnTypeChoice::Date => 1,
        view::ColumnTypeChoice::Ip => 2,
        view::ColumnTypeChoice::Float => 3,
        view::ColumnTypeChoice::Integer => 4,
        view::ColumnTypeChoice::SemVer => 5,
        view::ColumnTypeChoice::Boolean => 6,
    }
}

fn column_type_choice(index: usize) -> view::ColumnTypeChoice {
    match index {
        1 => view::ColumnTypeChoice::Date,
        2 => view::ColumnTypeChoice::Ip,
        3 => view::ColumnTypeChoice::Float,
        4 => view::ColumnTypeChoice::Integer,
        5 => view::ColumnTypeChoice::SemVer,
        6 => view::ColumnTypeChoice::Boolean,
        _ => view::ColumnTypeChoice::Text,
    }
}

fn column_format_index(choice: view::ColumnFormatChoice) -> usize {
    match choice {
        view::ColumnFormatChoice::Plain => 0,
        view::ColumnFormatChoice::Locale => 1,
        view::ColumnFormatChoice::Uppercase => 2,
        view::ColumnFormatChoice::Lowercase => 3,
        view::ColumnFormatChoice::Char => 4,
        view::ColumnFormatChoice::Bit => 5,
        view::ColumnFormatChoice::Word => 6,
    }
}

fn column_format_choice(index: usize) -> view::ColumnFormatChoice {
    match index {
        1 => view::ColumnFormatChoice::Locale,
        2 => view::ColumnFormatChoice::Uppercase,
        3 => view::ColumnFormatChoice::Lowercase,
        4 => view::ColumnFormatChoice::Char,
        5 => view::ColumnFormatChoice::Bit,
        6 => view::ColumnFormatChoice::Word,
        _ => view::ColumnFormatChoice::Plain,
    }
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

#[cfg(feature = "saved-views")]
fn apply_saved_view(
    config: &cli::Config,
    view: &mut view::TableView,
) -> anyhow::Result<Option<SavedViewRuntime>> {
    use crate::cli::SavedViewSelection as CliSavedViewSelection;
    use crate::ops::sort::{SortDirection, SortMode};
    use crate::saved_views::{self, FilterAction, SavedViewSelection, SortKind};

    let selection = match &config.saved_view {
        CliSavedViewSelection::Disabled => return Ok(None),
        CliSavedViewSelection::Auto => SavedViewSelection::Auto {
            input_path: &config.filename,
        },
        CliSavedViewSelection::Force(name) => SavedViewSelection::Force { name },
    };

    let target_path = placeholder_saved_view_path(&config.filename);
    let view_name = target_path
        .as_deref()
        .and_then(Path::file_stem)
        .and_then(|stem| stem.to_str())
        .unwrap_or("view")
        .to_owned();
    let discovered = saved_views::discover_saved_views(None);
    let mut messages = discovered
        .warnings
        .iter()
        .map(format_saved_view_warning)
        .collect::<Vec<_>>();
    let Some(selected) = saved_views::select_saved_view(&discovered.views, selection) else {
        if let CliSavedViewSelection::Force(name) = &config.saved_view {
            anyhow::bail!("saved view '{name}' was requested but was not found");
        }
        log_saved_view_messages(&messages);
        return Ok(Some(SavedViewRuntime {
            source_path: None,
            target_path,
            view_name,
            explicit_locale: None,
            messages,
        }));
    };
    messages.extend(selected.view.warnings.iter().map(format_saved_view_warning));
    messages.extend(selected.warnings.iter().map(format_saved_view_warning));
    let Some(header) = view.header() else {
        log_saved_view_messages(&messages);
        return Ok(Some(SavedViewRuntime {
            source_path: Some(selected.view.path.clone()),
            target_path: Some(selected.view.path.clone()),
            view_name: selected.view.canonical_name.clone(),
            explicit_locale: selected.view.view.locale.clone(),
            messages,
        }));
    };
    let header = header.to_vec();
    let resolved = saved_views::resolve_columns(&selected.view.view, &header);
    messages.extend(resolved.warnings.iter().map(format_saved_view_warning));
    view.apply_saved_columns(&resolved, selected.view.view.locale.as_deref());

    let sort_keys = selected
        .view
        .view
        .sort
        .iter()
        .filter_map(|sort| {
            let column = saved_views::resolve_column_reference(&header, &sort.column)?;
            let direction = match sort.direction {
                saved_views::SortDirection::Asc => SortDirection::Ascending,
                saved_views::SortDirection::Desc => SortDirection::Descending,
            };
            let mode = match sort.kind {
                SortKind::Lexical => SortMode::Lexical,
                SortKind::Natural => SortMode::Natural,
                SortKind::Numeric => SortMode::Numeric,
                SortKind::Type => view.type_sort_mode_for_source(column),
            };
            Some(view::ActiveSortKey {
                column,
                mode,
                direction,
            })
        })
        .collect::<Vec<_>>();
    view.apply_saved_sort_keys(sort_keys);

    for filter in &selected.view.view.filters {
        let Some(column) = saved_views::resolve_column_reference(&header, &filter.column) else {
            continue;
        };
        let mode = match filter.action {
            FilterAction::In => FilterMode::In,
            FilterAction::Out => FilterMode::Out,
        };
        let kind = match filter.kind {
            saved_views::FilterKind::Text => FilterKind::Text,
            saved_views::FilterKind::Regex => FilterKind::Regex,
            saved_views::FilterKind::Numeric => FilterKind::Numeric,
        };
        let _ = view.apply_source_filter(column, mode, kind, filter.condition.clone());
    }
    log_saved_view_messages(&messages);
    Ok(Some(SavedViewRuntime {
        source_path: Some(selected.view.path.clone()),
        target_path: Some(selected.view.path.clone()),
        view_name: selected.view.canonical_name.clone(),
        explicit_locale: selected.view.view.locale.clone(),
        messages,
    }))
}

#[cfg(feature = "saved-views")]
fn placeholder_saved_view_path(input: &Path) -> Option<PathBuf> {
    let view_dir = saved_views::saved_view_dir(None)?;
    let basename = input.file_name()?.to_str()?;
    let stem = if let Some((stem, _)) = basename.rsplit_once('.') {
        stem
    } else {
        basename
    };
    Some(view_dir.join(format!("{stem}.yml")))
}

#[cfg(feature = "saved-views")]
fn format_saved_view_warning(warning: &saved_views::SavedViewWarning) -> String {
    format!("saved view: {}: {}", warning.field, warning.message)
}

#[cfg(feature = "saved-views")]
fn log_saved_view_messages(messages: &[String]) {
    for message in messages {
        eprintln!("{message}");
    }
}

#[cfg(feature = "saved-views")]
fn write_saved_view_atomic(path: &Path, yaml: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::io::Write;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = if path.exists() {
        let existing = fs::read_to_string(path).unwrap_or_default();
        merge_saved_view_comments(&existing, yaml)
    } else {
        yaml.to_owned()
    };
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("yml")
    ));
    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(feature = "saved-views")]
fn merge_saved_view_comments(existing: &str, yaml: &str) -> String {
    let header = saved_view_header_comment_block(existing);
    let inline_comments = saved_view_inline_comments(existing);
    let mut merged = apply_saved_view_inline_comments(yaml, &inline_comments);
    if !header.is_empty() {
        merged = format!("{header}\n{merged}");
    }
    merged
}

#[cfg(feature = "saved-views")]
fn saved_view_header_comment_block(existing: &str) -> String {
    existing
        .lines()
        .take_while(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(feature = "saved-views")]
fn saved_view_inline_comments(existing: &str) -> std::collections::BTreeMap<String, String> {
    let mut tracker = SavedViewYamlPathTracker::default();
    let mut comments = std::collections::BTreeMap::new();
    for line in existing.lines() {
        let Some((content, comment)) = split_yaml_inline_comment(line) else {
            let _ = tracker.path_for_line(line);
            continue;
        };
        if let Some(path) = tracker.path_for_content(content) {
            let comment = comment.trim().to_owned();
            if let Some(wildcard_path) = saved_view_sequence_wildcard_path(&path) {
                comments.insert(wildcard_path, comment.clone());
            }
            comments.insert(path, comment);
        }
    }
    comments
}

#[cfg(feature = "saved-views")]
fn apply_saved_view_inline_comments(
    yaml: &str,
    comments: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut tracker = SavedViewYamlPathTracker::default();
    let mut output = String::new();
    for line in yaml.lines() {
        let mut line = line.to_owned();
        if let Some(path) = tracker.path_for_line(&line) {
            if let Some(comment) = comments.get(&path).or_else(|| {
                saved_view_sequence_wildcard_path(&path)
                    .as_ref()
                    .and_then(|path| comments.get(path))
            }) {
                line.push(' ');
                line.push_str(comment);
            }
        }
        output.push_str(&line);
        output.push('\n');
    }
    output
}

#[cfg(feature = "saved-views")]
fn saved_view_sequence_wildcard_path(path: &str) -> Option<String> {
    let (prefix, suffix) = path.rsplit_once('.')?;
    suffix
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| format!("{prefix}.*"))
}

#[cfg(feature = "saved-views")]
fn split_yaml_inline_comment(line: &str) -> Option<(&str, &str)> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        match ch {
            '\\' if in_double && !escaped => {
                escaped = true;
                continue;
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single && !escaped => in_double = !in_double,
            '#' if !in_single && !in_double => {
                let content = &line[..idx];
                if content.trim().is_empty() {
                    return None;
                }
                return Some((content.trim_end(), &line[idx..]));
            }
            _ => {}
        }
        escaped = false;
    }
    None
}

#[cfg(feature = "saved-views")]
#[derive(Default)]
struct SavedViewYamlPathTracker {
    stack: Vec<(usize, String)>,
    sequence_indices: std::collections::BTreeMap<String, usize>,
}

#[cfg(feature = "saved-views")]
impl SavedViewYamlPathTracker {
    fn path_for_line(&mut self, line: &str) -> Option<String> {
        let content = split_yaml_inline_comment(line)
            .map(|(content, _)| content)
            .unwrap_or(line);
        self.path_for_content(content)
    }

    fn path_for_content(&mut self, content: &str) -> Option<String> {
        let trimmed = content.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let indent = content.len().saturating_sub(trimmed.len());
        if let Some(rest) = trimmed.strip_prefix("- ") {
            self.stack
                .retain(|(stack_indent, _)| *stack_indent < indent);
            let parent = self.current_path();
            let index = self.sequence_indices.entry(parent).or_insert(0);
            let index_component = index.to_string();
            *index += 1;
            self.stack.push((indent, index_component));
            return self
                .path_for_mapping(rest)
                .or_else(|| Some(self.current_path()));
        }

        self.stack
            .retain(|(stack_indent, _)| *stack_indent < indent);
        self.path_for_mapping(trimmed)
    }

    fn path_for_mapping(&mut self, content: &str) -> Option<String> {
        let (key, value) = content.split_once(':')?;
        let key = saved_view_yaml_path_key(key);
        if key.is_empty() {
            return None;
        }
        let mut path = self.current_path();
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(&key);
        if value.trim().is_empty() {
            let indent = self
                .stack
                .last()
                .map(|(indent, _)| indent.saturating_add(2))
                .unwrap_or(0);
            self.stack.push((indent, key));
        }
        Some(path)
    }

    fn current_path(&self) -> String {
        self.stack
            .iter()
            .map(|(_, component)| component.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

#[cfg(feature = "saved-views")]
fn saved_view_yaml_path_key(key: &str) -> String {
    let key = key.trim();
    key.strip_prefix('"')
        .and_then(|key| key.strip_suffix('"'))
        .or_else(|| {
            key.strip_prefix('\'')
                .and_then(|key| key.strip_suffix('\''))
        })
        .unwrap_or(key)
        .to_owned()
}

fn closes_popup(event: KeyEvent) -> bool {
    matches!(event.code, KeyCode::Esc | KeyCode::Enter)
        || matches!(
            command::lookup_key_event(event),
            Some(command::Command::Quit | command::Command::Help)
        )
}

fn current_cell(view: &view::TableView) -> Option<String> {
    view.current_cell_rendered()
}

fn popup_area(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let width = (area.width.saturating_mul(3) / 4).max(20).min(area.width);
    let height = (area.height.saturating_mul(3) / 4).max(5).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    ratatui::layout::Rect::new(x, y, width, height)
}

fn table_area(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    ratatui::layout::Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1))
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
            column_info: None,
            search_query: String::new(),
            keys: command::KeyInterpreter::default(),
            message: None,
            theme: theme::default_theme(),
            #[cfg(feature = "saved-views")]
            saved_view: None,
            #[cfg(feature = "saved-views")]
            view_modal: None,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[cfg(feature = "saved-views")]
    fn app_with_saved_view_target(rows: Vec<Vec<String>>, target_path: PathBuf) -> App {
        App {
            source: ingest::source::InputSource::Path(PathBuf::from("cat_shards.txt")),
            parse_options: ingest::ParseOptions::default(),
            view: view::TableView::classify(rows, view::Viewport::new(10, 4)),
            popup: None,
            filter_prompt: None,
            column_info: None,
            search_query: String::new(),
            keys: command::KeyInterpreter::default(),
            message: None,
            theme: theme::default_theme(),
            saved_view: Some(SavedViewRuntime {
                source_path: None,
                target_path: Some(target_path),
                view_name: "cat_shards".to_owned(),
                explicit_locale: None,
                messages: Vec::new(),
            }),
            view_modal: None,
        }
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
    fn column_info_modal_edits_format_and_sort() {
        let mut app = app_with_rows(rows(&[&["Name"], &["beta"], &["alpha"]]));

        app.handle_key(key(KeyCode::Char('i')))
            .expect("column info");
        assert_eq!(app.popup, Some(ui::Popup::ColumnInfo));
        let popup = app.column_info.as_ref().expect("modal").popup();
        assert!(popup.summary.contains("Name"));
        assert!(popup
            .sections
            .iter()
            .any(|section| section.header == "Type"));
        assert!(popup
            .sections
            .iter()
            .find(|section| section.header == "Filters")
            .is_some_and(|section| section.options.is_empty() && section.details == ["None"]));

        for _ in 0..3 {
            app.handle_key(key(KeyCode::Tab)).expect("next group");
        }
        app.handle_key(key(KeyCode::Down)).expect("uppercase");
        app.handle_key(key(KeyCode::Tab)).expect("sort group");
        app.handle_key(key(KeyCode::Down)).expect("ascending");
        app.handle_key(key(KeyCode::Enter)).expect("save");

        assert_eq!(app.popup, None);
        assert_eq!(
            app.view.rendered_header().expect("header"),
            vec!["▲Name".to_owned()]
        );
        assert_eq!(app.view.visible_rows_vec(), rows(&[&["ALPHA"], &["BETA"]]));
    }

    #[test]
    fn column_info_modal_displays_and_clears_filters() {
        let mut app = app_with_rows(rows(&[&["Name"], &["alpha"], &["beta"]]));
        app.view
            .apply_filter(0, FilterMode::In, FilterKind::Text, "alp".to_owned())
            .expect("apply filter");

        app.handle_key(key(KeyCode::Char('i')))
            .expect("column info");
        let popup = app.column_info.as_ref().expect("modal").popup();
        assert!(popup
            .sections
            .iter()
            .flat_map(|section| section.details.iter())
            .any(|detail| detail.contains("in text alp")));

        for _ in 0..5 {
            app.handle_key(key(KeyCode::Tab)).expect("next group");
        }
        app.handle_key(key(KeyCode::Down)).expect("clear filters");
        app.handle_key(key(KeyCode::Enter)).expect("save");

        assert_eq!(app.popup, None);
        assert_eq!(app.view.row_count(), 2);
        assert!(!app.view.column_has_filter(0));
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
            column_info: None,
            search_query: String::new(),
            keys: command::KeyInterpreter::default(),
            message: None,
            theme: theme::default_theme(),
            #[cfg(feature = "saved-views")]
            saved_view: None,
            #[cfg(feature = "saved-views")]
            view_modal: None,
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

    #[cfg(feature = "saved-views")]
    #[test]
    fn saved_view_modal_displays_placeholder_and_saves_immediately() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("views").join("cat_shards.yml");
        let mut app = app_with_saved_view_target(rows(&[&["Name"], &["alpha"]]), target.clone());

        app.open_saved_view_modal();
        let modal = app.view_modal.as_ref().expect("modal");
        assert!(modal.filename.contains("cat_shards.yml"));
        assert!(modal.yaml.contains("name: cat_shards"));
        assert!(modal.yaml.contains("filenames:\n  - cat_shards.txt"));

        app.handle_saved_view_key(key(KeyCode::Char('s')));

        let saved = std::fs::read_to_string(&target).expect("saved file");
        assert!(saved.contains("name: cat_shards"));
        assert!(app
            .message
            .as_deref()
            .is_some_and(|message| message.contains("saved view")));
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn saved_view_modal_confirms_and_declines_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("cat_shards.yml");
        std::fs::write(
            &target,
            concat!(
                "# keep me\n",
                "# describe this view\n",
                "name: old # view name\n",
                "filenames: # filename patterns\n",
                "  - old # first filename\n",
            ),
        )
        .expect("write old");
        let mut app = app_with_saved_view_target(rows(&[&["Name"], &["alpha"]]), target.clone());
        app.open_saved_view_modal();

        app.handle_saved_view_key(key(KeyCode::Char('s')));
        assert!(app.view_modal.as_ref().expect("modal").confirming_overwrite);
        app.handle_saved_view_key(key(KeyCode::Char('n')));
        assert!(!app.view_modal.as_ref().expect("modal").confirming_overwrite);
        assert!(std::fs::read_to_string(&target)
            .expect("old")
            .contains("name: old"));

        app.handle_saved_view_key(key(KeyCode::Char('s')));
        app.handle_saved_view_key(key(KeyCode::Char('y')));
        let saved = std::fs::read_to_string(&target).expect("saved file");
        assert!(saved.starts_with("# keep me\n# describe this view\n"));
        assert!(saved.contains("name: cat_shards # view name"));
        assert!(saved.contains("filenames: # filename patterns"));
        assert!(
            saved.contains("  - cat_shards.txt # first filename"),
            "{saved}"
        );
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn saved_view_comment_merge_keeps_header_and_inline_comments() {
        let existing = "# header\nname: old # view name\nfilenames: # filename patterns\n  - old # first filename\n";
        let yaml = "name: cat_shards\nfilenames:\n  - cat_shards.txt\n";

        let merged = merge_saved_view_comments(existing, yaml);

        assert!(merged.starts_with("# header\n"));
        assert!(merged.contains("name: cat_shards # view name"));
        assert!(merged.contains("filenames: # filename patterns"));
        assert!(
            merged.contains("  - cat_shards.txt # first filename"),
            "{merged}"
        );
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn saved_view_modal_scrolls_and_reports_save_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().to_path_buf();
        let mut app = app_with_saved_view_target(
            rows(&[&["A", "B", "C"], &["1", "2", "3"], &["4", "5", "6"]]),
            target,
        );
        app.open_saved_view_modal();

        app.handle_saved_view_key(key(KeyCode::Char('j')));
        assert_eq!(app.view_modal.as_ref().expect("modal").scroll, 1);

        app.handle_saved_view_key(key(KeyCode::Char('s')));
        assert!(app.view_modal.as_ref().expect("modal").confirming_overwrite);
        app.handle_saved_view_key(key(KeyCode::Char('y')));

        assert_eq!(app.popup, Some(ui::Popup::SavedView));
        assert!(app
            .message
            .as_deref()
            .is_some_and(|message| message.contains("failed to save view")));
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn saved_view_binding_reports_disabled_when_no_view_context() {
        let mut app = app_with_rows(rows(&[&["Name"], &["alpha"]]));

        app.apply(command::KeyAction {
            command: command::Command::SavedView,
            count: None,
        })
        .expect("apply");

        assert_eq!(app.popup, None);
        assert_eq!(app.message.as_deref(), Some("saved views are disabled"));
    }
}
