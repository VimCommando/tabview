use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
    Path(PathBuf),
    Stdin,
    StreamingStdin(StreamingInput),
}

impl InputSource {
    pub fn from_cli_value(value: &str) -> Self {
        if value == "-" {
            return Self::Stdin;
        }
        Self::Path(parse_path(value))
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Path(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or("input"))
                .to_owned(),
            Self::Stdin | Self::StreamingStdin(_) => "stdin".to_owned(),
        }
    }

    pub fn is_seekable(&self) -> bool {
        matches!(self, Self::Path(_))
    }

    pub fn is_stdin(&self) -> bool {
        matches!(self, Self::Stdin | Self::StreamingStdin(_))
    }

    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::StreamingStdin(_))
    }
}

pub fn read_stdin() -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn read_source(source: &InputSource) -> io::Result<Vec<u8>> {
    match source {
        InputSource::Path(path) => std::fs::read(path),
        InputSource::Stdin => read_stdin(),
        InputSource::StreamingStdin(input) => input.snapshot(true).map(|snapshot| snapshot.bytes),
    }
}

#[derive(Debug, Clone)]
pub struct StreamSnapshot {
    pub bytes: Vec<u8>,
    pub complete: bool,
}

#[derive(Debug, Default)]
struct StreamingState {
    bytes: Vec<u8>,
    complete: bool,
    error: Option<(io::ErrorKind, String)>,
}

#[derive(Debug, Default)]
struct StreamingInner {
    state: Mutex<StreamingState>,
    changed: Condvar,
}

#[derive(Debug, Clone)]
pub struct StreamingInput {
    inner: Arc<StreamingInner>,
}

impl PartialEq for StreamingInput {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for StreamingInput {}

impl StreamingInput {
    pub fn snapshot(&self, wait_for_completion: bool) -> io::Result<StreamSnapshot> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while !state.complete && (wait_for_completion || state.bytes.is_empty()) {
            state = self
                .inner
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        if let Some((kind, message)) = &state.error {
            return Err(io::Error::new(*kind, message.clone()));
        }
        Ok(StreamSnapshot {
            bytes: state.bytes.clone(),
            complete: state.complete,
        })
    }

    pub fn wait_for_delimited_sample(&self) -> io::Result<()> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while !state.complete
            && state.bytes.len() < 64 * 1024
            && state.bytes.iter().filter(|byte| **byte == b'\n').count() < 2
        {
            state = self
                .inner
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        if let Some((kind, message)) = &state.error {
            Err(io::Error::new(*kind, message.clone()))
        } else {
            Ok(())
        }
    }

    pub fn wait_for_probe_sample(&self) -> io::Result<()> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while !state.complete && state.bytes.len() < 64 * 1024 && !probe_sample_ready(&state.bytes)
        {
            state = self
                .inner
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        if let Some((kind, message)) = &state.error {
            Err(io::Error::new(*kind, message.clone()))
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_for_test() -> Self {
        Self {
            inner: Arc::new(StreamingInner::default()),
        }
    }

    #[cfg(test)]
    pub(crate) fn append_for_test(&self, bytes: &[u8]) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.bytes.extend_from_slice(bytes);
        self.inner.changed.notify_all();
    }

    #[cfg(test)]
    pub(crate) fn finish_for_test(&self) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.complete = true;
        self.inner.changed.notify_all();
    }
}

fn probe_sample_ready(bytes: &[u8]) -> bool {
    let first = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(bytes)
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace());
    first == Some(b'[')
        || bytes.iter().filter(|byte| **byte == b'\n').count() >= 2
        || (!bytes.ends_with(b"\n") && serde_json::from_slice::<serde_json::Value>(bytes).is_ok())
}

pub fn stream_stdin_for_interactive() -> InputSource {
    stream_reader_for_interactive(Box::new(io::stdin()))
}

pub fn stream_reader_for_interactive(mut reader: Box<dyn Read + Send>) -> InputSource {
    let input = StreamingInput {
        inner: Arc::new(StreamingInner::default()),
    };
    let worker_input = input.clone();
    std::thread::spawn(move || {
        let mut chunk = vec![0_u8; 64 * 1024];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => {
                    let mut state = worker_input
                        .inner
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    state.complete = true;
                    worker_input.inner.changed.notify_all();
                    break;
                }
                Ok(count) => {
                    let mut state = worker_input
                        .inner
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    state.bytes.extend_from_slice(&chunk[..count]);
                    worker_input.inner.changed.notify_all();
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => {
                    let mut state = worker_input
                        .inner
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    state.error = Some((error.kind(), error.to_string()));
                    state.complete = true;
                    worker_input.inner.changed.notify_all();
                    break;
                }
            }
        }
    });

    InputSource::StreamingStdin(input)
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

    #[test]
    fn streaming_sources_compare_by_identity() {
        let input = StreamingInput {
            inner: Arc::new(StreamingInner::default()),
        };
        assert_eq!(input, input.clone());
        assert_ne!(
            input,
            StreamingInput {
                inner: Arc::new(StreamingInner::default())
            }
        );
    }

    #[test]
    fn automatic_probe_can_start_json_arrays_before_eof() {
        assert!(probe_sample_ready(br#"[{"a":1},"#));
    }

    #[test]
    fn automatic_probe_accepts_a_complete_single_json_value_without_a_newline() {
        assert!(probe_sample_ready(br#"{"a":1}"#));
    }

    #[test]
    fn automatic_probe_waits_long_enough_to_distinguish_ndjson() {
        assert!(!probe_sample_ready(b"{\"a\":1}\n"));
        assert!(probe_sample_ready(b"{\"a\":1}\n{\"a\":2}\n"));
    }
}
