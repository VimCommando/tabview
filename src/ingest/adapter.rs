use std::io::Read;
use std::path::Path;

use crate::table::{RelationMetadata, SourceGeneration, TableDefinition, TableStore};

use super::source::InputSource;
use super::{DelimitedAdapter, JsonAdapter};
use super::{InputFormat, ObjectMode, ObjectModeOrigin, ObjectModeResolution, OpenOptions};

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
    pub object_mode: Option<ObjectModeResolution>,
    pub warnings: Vec<String>,
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
    let detected = match &source {
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
        InputSource::StreamingStdin(input) => {
            if options.format != InputFormat::Auto {
                options.format
            } else {
                input.wait_for_probe_sample()?;
                let snapshot = input.snapshot(false)?;
                let sample_len = snapshot.bytes.len().min(64 * 1024);
                FormatResolver::resolve(options.format, &source, &snapshot.bytes[..sample_len])
            }
        }
    };
    // A JSON Pointer is itself an explicit request for structured parsing. If
    // auto-detection cannot identify JSON/NDJSON, prefer JSON so the option is
    // either honored or produces a parse error instead of being silently
    // ignored by the delimited adapter.
    let resolved = resolve_structured_options(detected, options);
    let incompatible_object_mode = options.object_mode != ObjectMode::Auto
        && matches!(resolved, InputFormat::Delimited | InputFormat::Ndjson);
    let mut effective_options = options.clone();
    let warning =
        if incompatible_object_mode && options.object_mode_origin == ObjectModeOrigin::SavedView {
            effective_options.object_mode = ObjectMode::Auto;
            Some(format!(
                "saved object_mode '{}' is incompatible with {resolved} input and was ignored",
                options.object_mode
            ))
        } else if incompatible_object_mode {
            anyhow::bail!(
                "object mode '{}' is incompatible with {resolved} input",
                options.object_mode
            );
        } else {
            None
        };
    let mut opened = match resolved {
        InputFormat::Delimited => DelimitedAdapter.open(source, &effective_options),
        InputFormat::Json => JsonAdapter::json().open(source, &effective_options),
        InputFormat::Ndjson => JsonAdapter::ndjson().open(source, &effective_options),
        InputFormat::Auto => unreachable!("auto format must be resolved"),
    }?;
    if let Some(warning) = warning {
        for table in &mut opened.tables {
            table.warnings.push(warning.clone());
        }
    }
    Ok(opened)
}

fn resolve_structured_options(detected: InputFormat, options: &OpenOptions) -> InputFormat {
    if options.format == InputFormat::Auto
        && options.json_path.is_some()
        && detected == InputFormat::Delimited
    {
        InputFormat::Json
    } else {
        detected
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
    } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
        InputFormat::Json
    } else {
        InputFormat::Delimited
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::table::RowCount;

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
            FormatResolver::resolve(InputFormat::Auto, &InputSource::Stdin, b"[1]\n[2]\n"),
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

    #[test]
    fn json_path_prevents_auto_format_from_falling_back_to_delimited() {
        let options = OpenOptions {
            json_path: Some("/rows".parse().unwrap()),
            ..OpenOptions::default()
        };
        let detected = FormatResolver::resolve(
            options.format,
            &InputSource::Stdin,
            b"not a structured sample",
        );
        assert_eq!(detected, InputFormat::Delimited);

        let resolved = resolve_structured_options(detected, &options);
        assert_eq!(resolved, InputFormat::Json);
    }

    #[test]
    fn automatic_streaming_probe_selects_ndjson_without_waiting_for_eof() {
        let input = crate::ingest::source::StreamingInput::pending_for_test();
        input.append_for_test(b"{\"a\":1}\n{\"a\":2}\n");
        let source = open_source(
            InputSource::StreamingStdin(input.clone()),
            &OpenOptions::default(),
        )
        .expect("open streaming NDJSON");
        let table = source.into_implicit_table().expect("table");
        assert_eq!(table.store.row_count(), RowCount::AtLeast(2));
        input.finish_for_test();
    }
}
