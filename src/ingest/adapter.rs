use std::io::Read;
use std::path::Path;

use crate::table::{RelationMetadata, SourceGeneration, TableDefinition, TableStore};

use super::source::InputSource;
use super::{DelimitedAdapter, JsonAdapter};
use super::{InputFormat, OpenOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeResult {
    NoMatch,
    Possible,
    Strong,
}

pub struct OpenedTable {
    pub generation: SourceGeneration,
    pub definition: TableDefinition,
    pub store: Box<dyn TableStore>,
}

pub struct OpenedSource {
    // Public multi-relation construction and selection are introduced with the
    // follow-on SQLite/database source rather than the file-adapter change.
    relations: Vec<RelationMetadata>,
    tables: Vec<OpenedTable>,
}

impl OpenedSource {
    pub fn implicit(table: OpenedTable) -> Self {
        Self {
            relations: vec![table.definition.relation.clone()],
            tables: vec![table],
        }
    }

    pub fn list_relations(&self) -> &[RelationMetadata] {
        &self.relations
    }

    pub fn into_implicit_table(mut self) -> anyhow::Result<OpenedTable> {
        if self.tables.len() != 1 {
            anyhow::bail!("source does not contain exactly one implicit relation");
        }
        Ok(self.tables.remove(0))
    }
}

pub trait SourceAdapter {
    fn format(&self) -> InputFormat;
    fn probe(&self, source: &InputSource, sample: &[u8]) -> ProbeResult;
    fn open(&self, source: InputSource, options: &OpenOptions) -> anyhow::Result<OpenedSource>;
}

pub struct FormatResolver;

impl FormatResolver {
    pub fn resolve(requested: InputFormat, source: &InputSource, sample: &[u8]) -> InputFormat {
        if requested != InputFormat::Auto {
            return requested;
        }

        if let InputSource::Path(path) = source {
            if let Some(format) = format_from_extension(path) {
                return format;
            }
        }

        probe_content(sample)
    }
}

pub fn open_source(source: InputSource, options: &OpenOptions) -> anyhow::Result<OpenedSource> {
    let resolved = match &source {
        InputSource::Path(path) => {
            let mut sample = Vec::new();
            std::fs::File::open(path)?
                .take(64 * 1024)
                .read_to_end(&mut sample)?;
            FormatResolver::resolve(options.format, &source, &sample)
        }
        InputSource::Stdin => {
            // Stdin is consumed exactly once by the selected adapter. With no
            // explicit or saved-view format, preserve the historical
            // delimited default rather than attempting a destructive probe.
            if options.format == InputFormat::Auto {
                InputFormat::Delimited
            } else {
                options.format
            }
        }
    };
    match resolved {
        InputFormat::Delimited => DelimitedAdapter.open(source, options),
        InputFormat::Json => JsonAdapter::json().open(source, options),
        InputFormat::Ndjson => JsonAdapter::ndjson().open(source, options),
        InputFormat::Auto => unreachable!("auto format must be resolved"),
    }
}

fn format_from_extension(path: &Path) -> Option<InputFormat> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "json" => Some(InputFormat::Json),
        "ndjson" | "jsonl" => Some(InputFormat::Ndjson),
        _ => None,
    }
}

fn probe_content(sample: &[u8]) -> InputFormat {
    let Ok(text) = std::str::from_utf8(sample) else {
        return InputFormat::Delimited;
    };
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    if trimmed.starts_with('[') {
        return InputFormat::Json;
    }
    let structured_lines = trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| {
            let line = line.trim();
            line.starts_with('{') || line.starts_with('[')
        })
        .count();
    let nonempty_lines = trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    if nonempty_lines > 1 && structured_lines == nonempty_lines {
        InputFormat::Ndjson
    } else if trimmed.starts_with('{') {
        InputFormat::Json
    } else {
        InputFormat::Delimited
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn explicit_format_has_precedence() {
        let source = InputSource::Path(PathBuf::from("data.json"));
        assert_eq!(
            FormatResolver::resolve(InputFormat::Delimited, &source, br#"[{"a":1}]"#),
            InputFormat::Delimited
        );
    }

    #[test]
    fn resolves_extensions_content_and_stdin() {
        assert_eq!(
            FormatResolver::resolve(
                InputFormat::Auto,
                &InputSource::Path(PathBuf::from("data.jsonl")),
                b""
            ),
            InputFormat::Ndjson
        );
        assert_eq!(
            FormatResolver::resolve(InputFormat::Auto, &InputSource::Stdin, b"{\"a\":1}\n"),
            InputFormat::Json
        );
        assert_eq!(
            FormatResolver::resolve(
                InputFormat::Auto,
                &InputSource::Stdin,
                b"{\"a\":1}\n{\"a\":2}\n"
            ),
            InputFormat::Ndjson
        );
        assert_eq!(
            FormatResolver::resolve(InputFormat::Auto, &InputSource::Stdin, b"a,b\n1,2\n"),
            InputFormat::Delimited
        );
    }

    #[test]
    fn explicit_selection_resolves_ambiguous_content() {
        let source = InputSource::Stdin;
        let sample = b"{a,b}\n";
        assert_eq!(
            FormatResolver::resolve(InputFormat::Json, &source, sample),
            InputFormat::Json
        );
        assert_eq!(
            FormatResolver::resolve(InputFormat::Delimited, &source, sample),
            InputFormat::Delimited
        );
    }
}
