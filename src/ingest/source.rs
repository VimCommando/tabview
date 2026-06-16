use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
    Path(PathBuf),
    Stdin,
}

impl InputSource {
    pub fn from_cli_value(value: &str) -> Self {
        if value == "-" {
            return Self::Stdin;
        }
        Self::Path(parse_path(value))
    }
}

pub fn read_stdin_then_restore_tty() -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    restore_stdin_from_tty()?;
    Ok(bytes)
}

pub fn read_source(source: &InputSource) -> io::Result<Vec<u8>> {
    match source {
        InputSource::Path(path) => std::fs::read(path),
        InputSource::Stdin => read_stdin_then_restore_tty(),
    }
}

#[cfg(unix)]
fn restore_stdin_from_tty() -> io::Result<()> {
    use std::fs::File;
    use std::os::fd::AsRawFd;

    let tty = File::open("/dev/tty")?;
    // SAFETY: dup2 is called with a valid file descriptor opened from /dev/tty
    // and the standard stdin descriptor. On success fd 0 refers to the tty.
    let result = unsafe { libc::dup2(tty.as_raw_fd(), libc::STDIN_FILENO) };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn restore_stdin_from_tty() -> io::Result<()> {
    Ok(())
}

fn parse_path(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("file://") {
        if let Some(path) = rest.strip_prefix("localhost/") {
            return PathBuf::from(format!("/{path}"));
        }
        return PathBuf::from(rest);
    }
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stdin_marker() {
        assert_eq!(InputSource::from_cli_value("-"), InputSource::Stdin);
    }

    #[test]
    fn parses_file_uri_path() {
        assert_eq!(
            InputSource::from_cli_value("file:///tmp/data.csv"),
            InputSource::Path(PathBuf::from("/tmp/data.csv"))
        );
        assert_eq!(
            InputSource::from_cli_value("file://localhost/tmp/data.csv"),
            InputSource::Path(PathBuf::from("/tmp/data.csv"))
        );
    }
}
