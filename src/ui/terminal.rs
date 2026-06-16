use std::io::{self, Stdout};
use std::marker::PhantomData;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[derive(Debug)]
pub struct Raw;

#[derive(Debug)]
pub struct Active;

#[derive(Debug)]
pub struct TerminalState<State> {
    state: PhantomData<State>,
}

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalState<Raw> {
    pub fn new() -> Self {
        Self { state: PhantomData }
    }

    pub fn activate(self) -> TerminalState<Active> {
        TerminalState { state: PhantomData }
    }
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

impl Default for TerminalState<Raw> {
    fn default() -> Self {
        Self::new()
    }
}
