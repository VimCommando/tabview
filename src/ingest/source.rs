use std::fmt;
use std::io::{self, Read};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex};

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceTarget {
    Path(PathBuf),
    Stdin,
    Url(Url),
    StreamingStdin(StreamingInput),
}

pub type InputSource = SourceTarget;

impl SourceTarget {
    pub fn from_cli_value(value: &str) -> Self {
        if value == "-" {
            return Self::Stdin;
        }
        if is_windows_drive_path(value) {
            return Self::Path(PathBuf::from(value));
        }
        match Url::parse(value) {
            Ok(url) if url.scheme() == "file" => url
                .to_file_path()
                .map(Self::Path)
                .unwrap_or_else(|_| Self::Path(PathBuf::from(value))),
            Ok(url) => Self::Url(url),
            Err(_) => Self::Path(PathBuf::from(value)),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Path(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or("input"))
                .to_owned(),
            Self::Stdin | Self::StreamingStdin(_) => "stdin".to_owned(),
            Self::Url(_) => self.safe_identity(),
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

    pub fn as_path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Path(path) => Some(path),
            Self::Stdin | Self::Url(_) | Self::StreamingStdin(_) => None,
        }
    }

    pub fn as_url(&self) -> Option<&Url> {
        match self {
            Self::Url(url) => Some(url),
            Self::Path(_) | Self::Stdin | Self::StreamingStdin(_) => None,
        }
    }

    /// A stable, non-secret representation suitable for diagnostics and
    /// saved-view matching. Query strings and fragments are intentionally
    /// omitted because endpoint URLs must not become a credential channel.
    pub fn safe_identity(&self) -> String {
        match self {
            Self::Path(path) => path.to_string_lossy().into_owned(),
            Self::Stdin | Self::StreamingStdin(_) => "-".to_owned(),
            Self::Url(url) => {
                let mut safe = url.clone();
                let _ = safe.set_username("");
                let _ = safe.set_password(None);
                safe.set_query(None);
                safe.set_fragment(None);
                safe.to_string()
            }
        }
    }

    /// A basename-safe identity for saved-view matching and generated YAML.
    /// Remote targets use the complete non-secret endpoint rather than only
    /// its final path segment so clusters on different hosts do not collide.
    pub fn saved_view_filename(&self) -> String {
        match self {
            Self::Path(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("input")
                .to_owned(),
            Self::Stdin | Self::StreamingStdin(_) => "-".to_owned(),
            Self::Url(_) => {
                let identity = self.safe_identity();
                let mut value = String::with_capacity(identity.len());
                let mut separator = false;
                for character in identity.chars() {
                    if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                        value.push(character);
                        separator = false;
                    } else if !separator && !value.is_empty() {
                        value.push('_');
                        separator = true;
                    }
                }
                value.trim_matches('_').to_owned()
            }
        }
    }
}

fn is_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

impl fmt::Display for SourceTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.safe_identity())
    }
}

impl FromStr for SourceTarget {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_cli_value(value))
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
        InputSource::Url(url) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "remote target '{}' is not a byte-stream source",
                safe_url(url)
            ),
        )),
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
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let first = bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace());
    let last_index = bytes.iter().rposition(|byte| !byte.is_ascii_whitespace());
    let last = last_index.map(|index| bytes[index]);
    let trailing_has_newline = last_index
        .map(|index| bytes[index + 1..].contains(&b'\n'))
        .unwrap_or(false);
    first == Some(b'[')
        || bytes.iter().filter(|byte| **byte == b'\n').count() >= 2
        || (first == Some(b'{') && last == Some(b'}') && !trailing_has_newline)
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

fn safe_url(url: &Url) -> String {
    SourceTarget::Url(url.clone()).safe_identity()
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
    fn parses_remote_urls_without_treating_them_as_paths() {
        let target = InputSource::from_cli_value("https://elastic.example:9200/logs");
        let url = target.as_url().expect("remote URL");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("elastic.example"));
        assert_eq!(url.port(), Some(9200));
        assert_eq!(url.path(), "/logs");
    }

    #[test]
    fn parses_windows_drive_paths_without_treating_the_drive_as_a_url_scheme() {
        for path in [r"C:/data/logs.csv", r"C:\data\logs.csv", "C:logs.csv"] {
            assert_eq!(
                InputSource::from_cli_value(path),
                InputSource::Path(PathBuf::from(path))
            );
        }
    }

    #[test]
    fn safe_remote_identity_redacts_secret_bearing_url_parts() {
        let target = InputSource::from_cli_value(
            "https://elastic:secret@elastic.example:9200/base?api_key=hidden#fragment",
        );
        assert_eq!(target.safe_identity(), "https://elastic.example:9200/base");
        assert!(!target.display_name().contains("secret"));
        assert!(!target.to_string().contains("hidden"));
        assert_eq!(
            target.saved_view_filename(),
            "https_elastic.example_9200_base"
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
        assert!(probe_sample_ready(b"\xEF\xBB\xBF  {\"a\":1} \t"));
    }

    #[test]
    fn automatic_probe_waits_for_an_incomplete_json_object() {
        assert!(!probe_sample_ready(br#"{"a":1"#));
    }

    #[test]
    fn automatic_probe_waits_long_enough_to_distinguish_ndjson() {
        assert!(!probe_sample_ready(b"{\"a\":1}\n"));
        assert!(probe_sample_ready(b"{\"a\":1}\n{\"a\":2}\n"));
    }
}
