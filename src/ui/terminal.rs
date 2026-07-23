#[cfg(windows)]
use std::io::Read;
use std::io::{self, Write};

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub(crate) struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Box<dyn Write>>>,
    restored: bool,
    #[cfg(windows)]
    windows_console: WindowsConsoleState,
}

impl TerminalSession {
    pub(crate) fn ensure_available() -> io::Result<()> {
        #[cfg(windows)]
        {
            return WindowsConsoleState::ensure_available();
        }
        #[cfg(not(windows))]
        terminal_writer().map(drop)
    }

    pub(crate) fn enter(preserve_data_stdin: bool) -> io::Result<Self> {
        #[cfg(not(windows))]
        let _ = preserve_data_stdin;
        #[cfg(windows)]
        let windows_console = WindowsConsoleState::attach(preserve_data_stdin)?;
        #[cfg(windows)]
        let output: Box<dyn Write> = Box::new(windows_console.output.try_clone()?);
        #[cfg(not(windows))]
        let output = terminal_writer()?;
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(output);
        let mut terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = disable_raw_mode();
                return Err(error);
            }
        };
        if let Err(error) = execute!(terminal.backend_mut(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        if let Err(error) = terminal.hide_cursor() {
            let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self {
            terminal,
            restored: false,
            #[cfg(windows)]
            windows_console,
        })
    }

    pub(crate) fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Box<dyn Write>>> {
        &mut self.terminal
    }

    pub(crate) fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        let raw_result = disable_raw_mode();
        let screen_result = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let cursor_result = self.terminal.show_cursor();
        #[cfg(windows)]
        let windows_result = Some(self.windows_console.restore());
        #[cfg(not(windows))]
        let windows_result: Option<io::Result<()>> = None;
        let result = first_error(
            [raw_result, screen_result, cursor_result]
                .into_iter()
                .chain(windows_result),
        );
        self.restored = result.is_ok();
        result
    }

    #[cfg(windows)]
    pub(crate) fn take_data_stdin_reader(&mut self) -> io::Result<Box<dyn Read + Send>> {
        self.windows_console
            .data_stdin
            .take()
            .map(|file| Box::new(file) as Box<dyn Read + Send>)
            .ok_or_else(|| io::Error::other("standard input was not preserved for table data"))
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn first_error(results: impl IntoIterator<Item = io::Result<()>>) -> io::Result<()> {
    results
        .into_iter()
        .find_map(Result::err)
        .map_or(Ok(()), Err)
}

#[cfg(unix)]
fn terminal_writer() -> io::Result<Box<dyn Write>> {
    use std::fs::OpenOptions;

    Ok(Box::new(
        OpenOptions::new().read(true).write(true).open("/dev/tty")?,
    ))
}

#[cfg(not(any(unix, windows)))]
fn terminal_writer() -> io::Result<Box<dyn Write>> {
    Ok(Box::new(io::stdout()))
}

#[cfg(windows)]
struct WindowsConsoleState {
    original_input: windows_sys::Win32::Foundation::HANDLE,
    original_output: windows_sys::Win32::Foundation::HANDLE,
    _input: std::fs::File,
    output: std::fs::File,
    data_stdin: Option<std::fs::File>,
    restored: bool,
}

#[cfg(windows)]
impl WindowsConsoleState {
    fn ensure_available() -> io::Result<()> {
        use std::fs::OpenOptions;

        OpenOptions::new().read(true).write(true).open("CONIN$")?;
        OpenOptions::new().read(true).write(true).open("CONOUT$")?;
        Ok(())
    }

    fn attach(preserve_data_stdin: bool) -> io::Result<Self> {
        use std::fs::OpenOptions;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Console::{
            GetStdHandle, SetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
        };

        let original_input = valid_windows_handle(unsafe { GetStdHandle(STD_INPUT_HANDLE) })?;
        let original_output = valid_windows_handle(unsafe { GetStdHandle(STD_OUTPUT_HANDLE) })?;
        let data_stdin = preserve_data_stdin
            .then(|| duplicate_file_handle(original_input))
            .transpose()?;
        let input = OpenOptions::new().read(true).write(true).open("CONIN$")?;
        let output = OpenOptions::new().read(true).write(true).open("CONOUT$")?;
        windows_bool(unsafe { SetStdHandle(STD_INPUT_HANDLE, input.as_raw_handle() as _) })?;
        if let Err(error) =
            windows_bool(unsafe { SetStdHandle(STD_OUTPUT_HANDLE, output.as_raw_handle() as _) })
        {
            let _ = windows_bool(unsafe { SetStdHandle(STD_INPUT_HANDLE, original_input) });
            return Err(error);
        }
        Ok(Self {
            original_input,
            original_output,
            _input: input,
            output,
            data_stdin,
            restored: false,
        })
    }

    fn restore(&mut self) -> io::Result<()> {
        use windows_sys::Win32::System::Console::{
            SetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
        };

        if self.restored {
            return Ok(());
        }
        let input = windows_bool(unsafe { SetStdHandle(STD_INPUT_HANDLE, self.original_input) });
        let output = windows_bool(unsafe { SetStdHandle(STD_OUTPUT_HANDLE, self.original_output) });
        let result = first_error([input, output]);
        self.restored = result.is_ok();
        result
    }
}

#[cfg(windows)]
impl Drop for WindowsConsoleState {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(windows)]
fn valid_windows_handle(
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> io::Result<windows_sys::Win32::Foundation::HANDLE> {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(handle)
    }
}

#[cfg(windows)]
fn windows_bool(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn duplicate_file_handle(
    source: windows_sys::Win32::Foundation::HANDLE,
) -> io::Result<std::fs::File> {
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use windows_sys::Win32::Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let process = unsafe { GetCurrentProcess() };
    let mut duplicated: HANDLE = std::ptr::null_mut();
    windows_bool(unsafe {
        DuplicateHandle(
            process,
            source,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    })?;
    Ok(unsafe { std::fs::File::from_raw_handle(duplicated as RawHandle) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restoration_attempts_every_step_and_returns_the_first_error() {
        let first = io::Error::other("raw");
        let error = first_error([Err(first), Ok(()), Err(io::Error::other("cursor"))])
            .expect_err("restore error");
        assert_eq!(error.to_string(), "raw");
    }
}
