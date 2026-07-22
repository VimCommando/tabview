use std::collections::HashMap;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::table::{
    CellValue, ColumnDefinition, ColumnId, ColumnSourceIdentity, InMemoryTable, IndexProgress,
    QueryExecution, RelationMetadata, Row, RowCount, RowId, RowIndex, RowVisitor, ScanDirection,
    ScanProgress, ScanRequest, SchemaDelta, SchemaState, SourceGeneration, TableDefinition,
    TableQuery, TableStore, TypeOrigin, TypeWidening,
};

use super::adapter::{OpenedSource, OpenedTable, ProbeResult, SourceAdapter};
use super::source::{read_source, InputSource};
use super::{InputFormat, JsonPointer, OpenOptions, SchemaScan};

#[derive(Debug, Clone, Copy)]
pub struct JsonAdapter {
    format: InputFormat,
}

impl JsonAdapter {
    pub fn json() -> Self {
        Self {
            format: InputFormat::Json,
        }
    }

    pub fn ndjson() -> Self {
        Self {
            format: InputFormat::Ndjson,
        }
    }
}

impl SourceAdapter for JsonAdapter {
    fn format(&self) -> InputFormat {
        self.format
    }

    fn probe(&self, _source: &InputSource, sample: &[u8]) -> ProbeResult {
        let Ok(text) = std::str::from_utf8(sample) else {
            return ProbeResult::NoMatch;
        };
        let trimmed = text.trim_start_matches('\u{feff}').trim_start();
        match self.format {
            InputFormat::Json if trimmed.starts_with(['{', '[']) => ProbeResult::Strong,
            InputFormat::Ndjson
                if trimmed
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .count()
                    > 1 =>
            {
                ProbeResult::Possible
            }
            _ => ProbeResult::NoMatch,
        }
    }

    fn open(&self, source: InputSource, options: &OpenOptions) -> anyhow::Result<OpenedSource> {
        options.validate()?;
        if let InputSource::Path(path) = &source {
            if std::fs::metadata(path)?.len() >= options.lazy_threshold_bytes {
                match self.format {
                    InputFormat::Json => {
                        return open_lazy_json(path, source.display_name(), options);
                    }
                    InputFormat::Ndjson => {
                        return open_lazy_ndjson(path, source.display_name(), options);
                    }
                    _ => {}
                }
            }
        }
        let bytes = read_source(&source)?;
        let rows = match self.format {
            InputFormat::Json => parse_json_rows(&bytes, options.json_path.as_ref())?,
            InputFormat::Ndjson => parse_ndjson_rows(&bytes, options.json_path.as_ref())?,
            _ => anyhow::bail!("JSON adapter requires json or ndjson format"),
        };
        open_json_rows(rows, source.display_name(), options)
    }
}

const JSON_SOURCE_SAMPLE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonSourceFingerprint {
    len: u64,
    sample_hash: u64,
}

fn json_source_fingerprint(path: &Path) -> anyhow::Result<JsonSourceFingerprint> {
    let metadata = std::fs::metadata(path)?;
    let len = metadata.len();
    let mut file = File::open(path)?;
    let mut hasher = DefaultHasher::new();
    len.hash(&mut hasher);
    let mut sample = vec![0_u8; JSON_SOURCE_SAMPLE_BYTES as usize];
    let first_len = file.read(&mut sample)?;
    sample[..first_len].hash(&mut hasher);
    if len > JSON_SOURCE_SAMPLE_BYTES {
        file.seek(SeekFrom::Start(
            len.saturating_sub(JSON_SOURCE_SAMPLE_BYTES),
        ))?;
        let last_len = file.read(&mut sample)?;
        sample[..last_len].hash(&mut hasher);
    }
    Ok(JsonSourceFingerprint {
        len,
        sample_hash: hasher.finish(),
    })
}

#[derive(Debug, Clone)]
struct FlatRow {
    cells: Vec<(String, CellValue, ColumnSourceIdentity)>,
    source_bytes: u64,
}

fn parse_json_rows(bytes: &[u8], pointer: Option<&JsonPointer>) -> anyhow::Result<Vec<FlatRow>> {
    super::streaming_json::select_json_rows(bytes, pointer)?
        .iter()
        .map(|raw| serde_json::from_str::<Value>(raw.get()).map_err(Into::into))
        .map(|value| value.and_then(|value| flatten_row(&value)))
        .collect()
}

fn parse_ndjson_rows(bytes: &[u8], pointer: Option<&JsonPointer>) -> anyhow::Result<Vec<FlatRow>> {
    let stream = serde_json::Deserializer::from_slice(bytes).into_iter::<Value>();
    let mut rows = Vec::new();
    for document in stream {
        let document = document?;
        let selected = resolve_pointer(&document, pointer)?;
        if !matches!(selected, Value::Object(_) | Value::Array(_)) {
            anyhow::bail!("JSON starting path does not identify an object or array");
        }
        rows.push(flatten_row(selected)?);
    }
    Ok(rows)
}

fn resolve_pointer<'a>(
    root: &'a Value,
    pointer: Option<&JsonPointer>,
) -> anyhow::Result<&'a Value> {
    let Some(pointer) = pointer else {
        return Ok(root);
    };
    let mut selected = root;
    for segment in pointer.segments() {
        selected = match selected {
            Value::Object(object) => object.get(segment),
            Value::Array(array) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| array.get(index)),
            _ => None,
        }
        .ok_or_else(|| {
            anyhow::anyhow!("JSON starting path '{}' was not found", pointer.as_str())
        })?;
    }
    Ok(selected)
}

fn flatten_row(value: &Value) -> anyhow::Result<FlatRow> {
    let source_bytes = serde_json::to_vec(value)?.len() as u64;
    let mut cells = Vec::new();
    match value {
        Value::Object(object) => flatten_object(object, "", &mut cells)?,
        Value::Array(array) => {
            for (index, value) in array.iter().enumerate() {
                let path = format!("/{index}");
                cells.push((
                    path,
                    cell_from_value(value)?,
                    ColumnSourceIdentity::Positional(index),
                ));
            }
        }
        scalar => cells.push((
            "/0".to_owned(),
            cell_from_value(scalar)?,
            ColumnSourceIdentity::Positional(0),
        )),
    }
    Ok(FlatRow {
        cells,
        source_bytes,
    })
}

fn flatten_object(
    object: &serde_json::Map<String, Value>,
    prefix: &str,
    cells: &mut Vec<(String, CellValue, ColumnSourceIdentity)>,
) -> anyhow::Result<()> {
    for (key, value) in object {
        let path = format!("{prefix}/{}", JsonPointer::encode_segment(key));
        if let Value::Object(nested) = value {
            if nested.is_empty() {
                let pointer: JsonPointer = path.parse()?;
                cells.push((
                    path,
                    CellValue::Json("{}".to_owned()),
                    ColumnSourceIdentity::JsonPointer(pointer),
                ));
            } else {
                flatten_object(nested, &path, cells)?;
            }
        } else {
            let pointer: JsonPointer = path.parse()?;
            cells.push((
                path,
                cell_from_value(value)?,
                ColumnSourceIdentity::JsonPointer(pointer),
            ));
        }
    }
    Ok(())
}

fn cell_from_value(value: &Value) -> anyhow::Result<CellValue> {
    Ok(match value {
        Value::Null => CellValue::Null,
        Value::Bool(value) => CellValue::Boolean(*value),
        Value::Number(value) => value
            .as_i64()
            .map(CellValue::Integer)
            .or_else(|| value.as_f64().map(CellValue::Float))
            .ok_or_else(|| anyhow::anyhow!("JSON number cannot be represented"))?,
        Value::String(value) => CellValue::Text(value.clone()),
        Value::Array(_) | Value::Object(_) => CellValue::Json(serde_json::to_string(value)?),
    })
}

fn open_json_rows(
    rows: Vec<FlatRow>,
    display_name: String,
    options: &OpenOptions,
) -> anyhow::Result<OpenedSource> {
    let generation = SourceGeneration::new();
    let mut schema = JsonSchema::new(generation);
    let mut bytes_scanned = 0_u64;
    let initial_rows = match options.schema_scan {
        SchemaScan::Full => rows.len(),
        SchemaScan::Default => {
            let mut count = 0;
            for row in &rows {
                schema.observe(row);
                bytes_scanned = bytes_scanned.saturating_add(row.source_bytes);
                count += 1;
                if bytes_scanned >= options.schema_scan_bytes {
                    break;
                }
            }
            count
        }
    };
    if options.schema_scan == SchemaScan::Full {
        for row in &rows {
            schema.observe(row);
            bytes_scanned = bytes_scanned.saturating_add(row.source_bytes);
        }
    }
    schema.assign_initial_labels();
    let complete = initial_rows >= rows.len();
    let definition = TableDefinition {
        generation,
        columns: schema.columns.clone(),
        schema_state: if complete {
            SchemaState::Complete
        } else {
            SchemaState::Provisional
        },
        relation: RelationMetadata::implicit(display_name, true),
    };
    let store = JsonTableStore {
        generation,
        rows,
        schema,
        indexed_rows: initial_rows,
        bytes_scanned,
        complete,
    };
    Ok(OpenedSource::implicit(OpenedTable {
        generation,
        definition,
        store: Box::new(store),
    }))
}

#[derive(Debug, Clone)]
struct JsonSchema {
    generation: SourceGeneration,
    columns: Vec<ColumnDefinition>,
    indices: HashMap<String, usize>,
    labels_assigned: bool,
}

impl JsonSchema {
    fn new(generation: SourceGeneration) -> Self {
        Self {
            generation,
            columns: Vec::new(),
            indices: HashMap::new(),
            labels_assigned: false,
        }
    }

    fn observe(&mut self, row: &FlatRow) -> SchemaDelta {
        let mut delta = SchemaDelta::default();
        for (path, value, identity) in &row.cells {
            if let Some(index) = self.indices.get(path).copied() {
                let widened = self.columns[index].source_type.widen(value.logical_type());
                if widened != self.columns[index].source_type {
                    self.columns[index].source_type = widened;
                    delta.widened_types.push(TypeWidening {
                        column: self.columns[index].id,
                        source_type: widened,
                    });
                }
                continue;
            }
            let ordinal = self.columns.len();
            let mut column = ColumnDefinition {
                id: ColumnId {
                    generation: self.generation,
                    ordinal: ordinal as u32,
                },
                source_identity: identity.clone(),
                display_name: path.clone(),
                source_type: value.logical_type(),
                type_origin: TypeOrigin::Inferred,
            };
            if self.labels_assigned {
                column.display_name = shortest_nonconflicting_label(
                    path,
                    self.columns
                        .iter()
                        .map(|column| column.display_name.as_str()),
                );
            }
            self.indices.insert(path.clone(), ordinal);
            self.columns.push(column.clone());
            delta.added_columns.push(column);
        }
        delta
    }

    fn assign_initial_labels(&mut self) {
        let paths = self.indices.keys().cloned().collect::<Vec<_>>();
        for column in &mut self.columns {
            column.display_name = match &column.source_identity {
                ColumnSourceIdentity::Positional(index) => format!("Column {}", index + 1),
                _ => shortest_unique_label(
                    source_path(&column.source_identity),
                    paths.iter().map(String::as_str),
                ),
            };
        }
        self.labels_assigned = true;
    }
}

fn source_path(identity: &ColumnSourceIdentity) -> &str {
    match identity {
        ColumnSourceIdentity::JsonPointer(pointer) => pointer.as_str(),
        ColumnSourceIdentity::Positional(_) | ColumnSourceIdentity::Delimited { .. } => "",
    }
}

fn shortest_unique_label<'a>(path: &str, all: impl Iterator<Item = &'a str>) -> String {
    let paths = all.collect::<Vec<_>>();
    let segments = label_segments(path);
    for depth in 1..=segments.len().max(1) {
        let candidate = join_label_suffix(&segments, depth);
        let collisions = paths
            .iter()
            .filter(|other| join_label_suffix(&label_segments(other), depth) == candidate)
            .count();
        if collisions <= 1 {
            return candidate;
        }
    }
    path.to_owned()
}

fn shortest_nonconflicting_label<'a>(path: &str, used: impl Iterator<Item = &'a str>) -> String {
    let used = used.collect::<Vec<_>>();
    let segments = label_segments(path);
    for depth in 1..=segments.len().max(1) {
        let candidate = join_label_suffix(&segments, depth);
        if !used.iter().any(|label| *label == candidate) {
            return candidate;
        }
    }
    path.to_owned()
}

fn label_segments(path: &str) -> Vec<String> {
    path.parse::<JsonPointer>()
        .map(|pointer| {
            pointer
                .segments()
                .iter()
                .map(|segment| friendly_segment(segment))
                .collect()
        })
        .unwrap_or_else(|_| vec![path.to_owned()])
}

fn friendly_segment(segment: &str) -> String {
    if !segment.is_empty()
        && segment
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-'))
    {
        segment.to_owned()
    } else {
        format!(
            "[{}]",
            serde_json::to_string(segment).unwrap_or_else(|_| "\"?\"".to_owned())
        )
    }
}

fn join_label_suffix(segments: &[String], depth: usize) -> String {
    let start = segments.len().saturating_sub(depth);
    let mut label = String::new();
    for segment in &segments[start..] {
        if !label.is_empty() && !segment.starts_with('[') {
            label.push('.');
        }
        label.push_str(segment);
    }
    if label.is_empty() {
        "value".to_owned()
    } else {
        label
    }
}

fn open_lazy_json(
    path: &Path,
    display_name: String,
    options: &OpenOptions,
) -> anyhow::Result<OpenedSource> {
    let mut file = File::open(path)?;
    let selected_offset = locate_json_pointer(&mut file, options.json_path.as_ref())?;
    file.seek(SeekFrom::Start(selected_offset))?;
    let selected_kind = read_non_whitespace(&mut file)?
        .ok_or_else(|| anyhow::anyhow!("JSON starting path was not found"))?;
    if selected_kind == b'{' {
        file.seek(SeekFrom::Start(selected_offset))?;
        let mut deserializer = serde_json::Deserializer::from_reader(file);
        let value = Value::deserialize(&mut deserializer)?;
        return open_json_rows(vec![flatten_row(&value)?], display_name, options);
    }
    if selected_kind != b'[' {
        anyhow::bail!("JSON starting path does not identify an object or array");
    }

    let generation = SourceGeneration::new();
    let mut store = LazyJsonArrayTable {
        generation,
        path: path.to_path_buf(),
        offsets: Vec::new(),
        schema: JsonSchema::new(generation),
        scan_offset: file.stream_position()?,
        bytes_scanned: 0,
        eof: false,
        fingerprint: json_source_fingerprint(path)?,
    };
    match options.schema_scan {
        SchemaScan::Full => {
            store.index_until(None, Some(u64::MAX))?;
        }
        SchemaScan::Default => {
            store.index_until(None, Some(options.schema_scan_bytes))?;
        }
    }
    store.schema.assign_initial_labels();
    let definition = TableDefinition {
        generation,
        columns: store.schema.columns.clone(),
        schema_state: if store.eof {
            SchemaState::Complete
        } else {
            SchemaState::Provisional
        },
        relation: RelationMetadata::implicit(display_name, true),
    };
    Ok(OpenedSource::implicit(OpenedTable {
        generation,
        definition,
        store: Box::new(store),
    }))
}

fn locate_json_pointer(file: &mut File, pointer: Option<&JsonPointer>) -> anyhow::Result<u64> {
    file.seek(SeekFrom::Start(0))?;
    locate_json_segments(file, pointer.map(JsonPointer::segments).unwrap_or_default())
}

fn locate_json_segments(file: &mut File, segments: &[String]) -> anyhow::Result<u64> {
    let value_offset = skip_whitespace(file)?
        .ok_or_else(|| anyhow::anyhow!("JSON starting path was not found"))?;
    if segments.is_empty() {
        return Ok(value_offset);
    }
    file.seek(SeekFrom::Start(value_offset))?;
    match read_byte(file)? {
        Some(b'{') => {
            loop {
                let Some(offset) = skip_whitespace(file)? else {
                    anyhow::bail!("unterminated JSON object");
                };
                file.seek(SeekFrom::Start(offset))?;
                if read_byte(file)? == Some(b'}') {
                    break;
                }
                file.seek(SeekFrom::Start(offset))?;
                let key = read_json_string(file)?;
                expect_json_byte(file, b':')?;
                if key == segments[0] {
                    return locate_json_segments(file, &segments[1..]);
                }
                skip_json_value(file)?;
                match read_non_whitespace(file)? {
                    Some(b',') => continue,
                    Some(b'}') => break,
                    _ => anyhow::bail!("invalid JSON object separator"),
                }
            }
            anyhow::bail!("JSON starting path was not found")
        }
        Some(b'[') => {
            let target = segments[0]
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("JSON Pointer array segment is not an index"))?;
            for index in 0..=target {
                let Some(offset) = skip_whitespace(file)? else {
                    anyhow::bail!("JSON starting path was not found");
                };
                file.seek(SeekFrom::Start(offset))?;
                if read_byte(file)? == Some(b']') {
                    anyhow::bail!("JSON starting path was not found");
                }
                file.seek(SeekFrom::Start(offset))?;
                if index == target {
                    return locate_json_segments(file, &segments[1..]);
                }
                skip_json_value(file)?;
                if read_non_whitespace(file)? != Some(b',') {
                    anyhow::bail!("JSON starting path was not found");
                }
            }
            unreachable!()
        }
        _ => anyhow::bail!("JSON starting path was not found"),
    }
}

fn read_byte(file: &mut File) -> anyhow::Result<Option<u8>> {
    let mut byte = [0_u8; 1];
    Ok((file.read(&mut byte)? == 1).then_some(byte[0]))
}

fn skip_whitespace(file: &mut File) -> anyhow::Result<Option<u64>> {
    loop {
        let offset = file.stream_position()?;
        match read_byte(file)? {
            Some(byte) if byte.is_ascii_whitespace() => continue,
            Some(_) => {
                file.seek(SeekFrom::Start(offset))?;
                return Ok(Some(offset));
            }
            None => return Ok(None),
        }
    }
}

fn read_non_whitespace(file: &mut File) -> anyhow::Result<Option<u8>> {
    let Some(offset) = skip_whitespace(file)? else {
        return Ok(None);
    };
    file.seek(SeekFrom::Start(offset))?;
    read_byte(file)
}

fn expect_json_byte(file: &mut File, expected: u8) -> anyhow::Result<()> {
    match read_non_whitespace(file)? {
        Some(actual) if actual == expected => Ok(()),
        _ => anyhow::bail!("invalid JSON syntax"),
    }
}

fn read_json_string(file: &mut File) -> anyhow::Result<String> {
    let start = skip_whitespace(file)?.ok_or_else(|| anyhow::anyhow!("expected JSON string"))?;
    file.seek(SeekFrom::Start(start))?;
    if read_byte(file)? != Some(b'\"') {
        anyhow::bail!("expected JSON object key");
    }
    let mut escaped = false;
    loop {
        let byte = read_byte(file)?.ok_or_else(|| anyhow::anyhow!("unterminated JSON string"))?;
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'\"' {
            break;
        }
    }
    let end = file.stream_position()?;
    let mut encoded = vec![0_u8; end.saturating_sub(start) as usize];
    file.seek(SeekFrom::Start(start))?;
    file.read_exact(&mut encoded)?;
    file.seek(SeekFrom::Start(end))?;
    Ok(serde_json::from_slice(&encoded)?)
}

fn skip_json_value(file: &mut File) -> anyhow::Result<(u64, u64)> {
    let start = skip_whitespace(file)?.ok_or_else(|| anyhow::anyhow!("expected JSON value"))?;
    file.seek(SeekFrom::Start(start))?;
    let first = read_byte(file)?.ok_or_else(|| anyhow::anyhow!("expected JSON value"))?;
    match first {
        b'\"' => {
            let mut escaped = false;
            loop {
                let byte =
                    read_byte(file)?.ok_or_else(|| anyhow::anyhow!("unterminated JSON string"))?;
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'\"' {
                    break;
                }
            }
        }
        b'{' | b'[' => {
            let mut stack = vec![first];
            while !stack.is_empty() {
                let byte = read_byte(file)?
                    .ok_or_else(|| anyhow::anyhow!("unterminated JSON container"))?;
                match byte {
                    b'\"' => {
                        let mut escaped = false;
                        loop {
                            let byte = read_byte(file)?
                                .ok_or_else(|| anyhow::anyhow!("unterminated JSON string"))?;
                            if escaped {
                                escaped = false;
                            } else if byte == b'\\' {
                                escaped = true;
                            } else if byte == b'\"' {
                                break;
                            }
                        }
                    }
                    b'{' | b'[' => stack.push(byte),
                    b'}' if stack.last() == Some(&b'{') => {
                        stack.pop();
                    }
                    b']' if stack.last() == Some(&b'[') => {
                        stack.pop();
                    }
                    _ => {}
                }
            }
        }
        _ => loop {
            let offset = file.stream_position()?;
            match read_byte(file)? {
                Some(byte) if byte.is_ascii_whitespace() || matches!(byte, b',' | b']' | b'}') => {
                    file.seek(SeekFrom::Start(offset))?;
                    break;
                }
                Some(_) => {}
                None => break,
            }
        },
    }
    Ok((start, file.stream_position()?))
}

#[derive(Debug, Clone)]
struct LazyJsonArrayTable {
    generation: SourceGeneration,
    path: PathBuf,
    offsets: Vec<u64>,
    schema: JsonSchema,
    scan_offset: u64,
    bytes_scanned: u64,
    eof: bool,
    fingerprint: JsonSourceFingerprint,
}

impl LazyJsonArrayTable {
    fn ensure_source_unchanged(&self) -> anyhow::Result<()> {
        if json_source_fingerprint(&self.path)? != self.fingerprint {
            anyhow::bail!("source changed during incremental access; reload is required");
        }
        Ok(())
    }

    fn read_flat_row(&self, index: usize) -> anyhow::Result<Option<FlatRow>> {
        let Some(offset) = self.offsets.get(index).copied() else {
            return Ok(None);
        };
        self.ensure_source_unchanged()?;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut deserializer = serde_json::Deserializer::from_reader(file);
        let value = Value::deserialize(&mut deserializer)?;
        Ok(Some(flatten_row(&value)?))
    }

    fn typed_row(&self, index: usize) -> anyhow::Result<Option<Row>> {
        Ok(self
            .read_flat_row(index)?
            .map(|flat| typed_row_from_flat(self.generation, index, &flat, &self.schema)))
    }

    fn index_until(
        &mut self,
        target: Option<usize>,
        byte_limit: Option<u64>,
    ) -> anyhow::Result<SchemaDelta> {
        if self.eof
            || target.is_some_and(|target| self.offsets.len() > target)
            || byte_limit.is_some_and(|limit| self.bytes_scanned >= limit)
        {
            return Ok(SchemaDelta::default());
        }
        self.ensure_source_unchanged()?;
        let mut file = File::open(&self.path)?;
        let mut delta = SchemaDelta::default();
        loop {
            if target.is_some_and(|target| self.offsets.len() > target)
                || byte_limit.is_some_and(|limit| self.bytes_scanned >= limit)
            {
                break;
            }
            file.seek(SeekFrom::Start(self.scan_offset))?;
            let Some(start) = skip_whitespace(&mut file)? else {
                anyhow::bail!("unterminated selected JSON array");
            };
            file.seek(SeekFrom::Start(start))?;
            if read_byte(&mut file)? == Some(b']') {
                self.scan_offset = file.stream_position()?;
                self.eof = true;
                delta.completed = true;
                break;
            }
            file.seek(SeekFrom::Start(start))?;
            let (_, end) = skip_json_value(&mut file)?;
            file.seek(SeekFrom::Start(start))?;
            let mut deserializer = serde_json::Deserializer::from_reader(&mut file);
            let value = Value::deserialize(&mut deserializer)?;
            let flat = flatten_row(&value)?;
            let observed = self.schema.observe(&flat);
            delta.added_columns.extend(observed.added_columns);
            delta.widened_types.extend(observed.widened_types);
            self.offsets.push(start);
            self.bytes_scanned = self.bytes_scanned.saturating_add(end.saturating_sub(start));

            file.seek(SeekFrom::Start(end))?;
            match read_non_whitespace(&mut file)? {
                Some(b',') => self.scan_offset = file.stream_position()?,
                Some(b']') => {
                    self.scan_offset = file.stream_position()?;
                    self.eof = true;
                    delta.completed = true;
                }
                _ => anyhow::bail!("invalid selected JSON array separator"),
            }
            self.ensure_source_unchanged()?;
            if self.eof {
                break;
            }
        }
        Ok(delta)
    }
}

impl TableStore for LazyJsonArrayTable {
    fn generation(&self) -> SourceGeneration {
        self.generation
    }

    fn row_count(&self) -> RowCount {
        if self.eof {
            RowCount::Exact(self.offsets.len())
        } else if self.offsets.is_empty() {
            RowCount::Unknown
        } else {
            RowCount::AtLeast(self.offsets.len())
        }
    }

    fn column_count(&self) -> usize {
        self.schema.columns.len()
    }

    fn row(&mut self, index: RowIndex) -> anyhow::Result<Option<Row>> {
        self.index_until(Some(index.0), None)?;
        self.typed_row(index.0)
    }

    fn ensure_indexed_through(&mut self, index: RowIndex) -> anyhow::Result<IndexProgress> {
        let delta = self.index_until(Some(index.0), None)?;
        Ok(IndexProgress {
            row_count: self.row_count(),
            schema_delta: delta,
            bytes_scanned: self.bytes_scanned,
        })
    }

    fn scan_rows(
        &mut self,
        request: ScanRequest,
        visitor: &mut dyn RowVisitor,
    ) -> anyhow::Result<ScanProgress> {
        let mut current = request.start.0;
        let mut visited = 0;
        while visited < request.max_rows {
            let Some(row) = self.row(RowIndex(current))? else {
                break;
            };
            visited += 1;
            if visitor.visit(RowIndex(current), &row).is_break() {
                return Ok(ScanProgress {
                    visited,
                    next: None,
                    reached_end: false,
                });
            }
            match request.direction {
                ScanDirection::Forward => current += 1,
                ScanDirection::Reverse if current > 0 => current -= 1,
                ScanDirection::Reverse => {
                    return Ok(ScanProgress {
                        visited,
                        next: None,
                        reached_end: true,
                    });
                }
            }
        }
        let reached_end = self.eof && current >= self.offsets.len();
        Ok(ScanProgress {
            visited,
            next: (!reached_end).then_some(RowIndex(current)),
            reached_end,
        })
    }

    fn materialize(&mut self) -> anyhow::Result<InMemoryTable> {
        self.index_until(None, Some(u64::MAX))?;
        InMemoryTable::from_rows(
            self.generation,
            (0..self.offsets.len())
                .map(|index| {
                    self.typed_row(index)?
                        .ok_or_else(|| anyhow::anyhow!("indexed JSON row is unavailable"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        )
    }
}

fn open_lazy_ndjson(
    path: &Path,
    display_name: String,
    options: &OpenOptions,
) -> anyhow::Result<OpenedSource> {
    let generation = SourceGeneration::new();
    let mut store = LazyNdjsonTable {
        generation,
        path: path.to_path_buf(),
        pointer: options.json_path.clone(),
        offsets: Vec::new(),
        schema: JsonSchema::new(generation),
        scan_offset: 0,
        eof: false,
        fingerprint: json_source_fingerprint(path)?,
    };
    match options.schema_scan {
        SchemaScan::Full => {
            store.index_until(None, Some(u64::MAX))?;
        }
        SchemaScan::Default => {
            store.index_until(None, Some(options.schema_scan_bytes))?;
        }
    }
    store.schema.assign_initial_labels();
    let definition = TableDefinition {
        generation,
        columns: store.schema.columns.clone(),
        schema_state: if store.eof {
            SchemaState::Complete
        } else {
            SchemaState::Provisional
        },
        relation: RelationMetadata::implicit(display_name, true),
    };
    Ok(OpenedSource::implicit(OpenedTable {
        generation,
        definition,
        store: Box::new(store),
    }))
}

#[derive(Debug, Clone)]
struct LazyNdjsonTable {
    generation: SourceGeneration,
    path: PathBuf,
    pointer: Option<JsonPointer>,
    offsets: Vec<u64>,
    schema: JsonSchema,
    scan_offset: u64,
    eof: bool,
    fingerprint: JsonSourceFingerprint,
}

impl LazyNdjsonTable {
    fn ensure_source_unchanged(&self) -> anyhow::Result<()> {
        if json_source_fingerprint(&self.path)? != self.fingerprint {
            anyhow::bail!("source changed during incremental access; reload is required");
        }
        Ok(())
    }

    fn index_until(
        &mut self,
        target: Option<usize>,
        byte_limit: Option<u64>,
    ) -> anyhow::Result<SchemaDelta> {
        if self.eof
            || target.is_some_and(|target| self.offsets.len() > target)
            || byte_limit.is_some_and(|limit| self.scan_offset >= limit)
        {
            return Ok(SchemaDelta::default());
        }
        self.ensure_source_unchanged()?;
        let base_offset = self.scan_offset;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(base_offset))?;
        let mut stream = serde_json::Deserializer::from_reader(file).into_iter::<Value>();
        let mut delta = SchemaDelta::default();

        loop {
            if target.is_some_and(|target| self.offsets.len() > target)
                || byte_limit.is_some_and(|limit| self.scan_offset >= limit)
            {
                break;
            }
            let document_offset = base_offset.saturating_add(stream.byte_offset() as u64);
            let Some(document) = stream.next() else {
                self.scan_offset = base_offset.saturating_add(stream.byte_offset() as u64);
                self.eof = true;
                if !matches!(self.schema.columns.as_slice(), []) {
                    delta.completed = true;
                }
                break;
            };
            let document = document?;
            let selected = resolve_pointer(&document, self.pointer.as_ref())?;
            if !matches!(selected, Value::Object(_) | Value::Array(_)) {
                anyhow::bail!("JSON starting path does not identify an object or array");
            }
            let flat = flatten_row(selected)?;
            let observed = self.schema.observe(&flat);
            delta.added_columns.extend(observed.added_columns);
            delta.widened_types.extend(observed.widened_types);
            self.offsets.push(document_offset);
            self.scan_offset = base_offset.saturating_add(stream.byte_offset() as u64);
            self.ensure_source_unchanged()?;
        }
        Ok(delta)
    }

    fn read_flat_row(&self, index: usize) -> anyhow::Result<Option<FlatRow>> {
        let Some(offset) = self.offsets.get(index).copied() else {
            return Ok(None);
        };
        self.ensure_source_unchanged()?;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut deserializer = serde_json::Deserializer::from_reader(file);
        let document = Value::deserialize(&mut deserializer)?;
        let selected = resolve_pointer(&document, self.pointer.as_ref())?;
        Ok(Some(flatten_row(selected)?))
    }

    fn typed_row(&self, index: usize) -> anyhow::Result<Option<Row>> {
        Ok(self
            .read_flat_row(index)?
            .map(|flat| typed_row_from_flat(self.generation, index, &flat, &self.schema)))
    }
}

impl TableStore for LazyNdjsonTable {
    fn generation(&self) -> SourceGeneration {
        self.generation
    }

    fn row_count(&self) -> RowCount {
        if self.eof {
            RowCount::Exact(self.offsets.len())
        } else if self.offsets.is_empty() {
            RowCount::Unknown
        } else {
            RowCount::AtLeast(self.offsets.len())
        }
    }

    fn column_count(&self) -> usize {
        self.schema.columns.len()
    }

    fn row(&mut self, index: RowIndex) -> anyhow::Result<Option<Row>> {
        self.index_until(Some(index.0), None)?;
        self.typed_row(index.0)
    }

    fn ensure_indexed_through(&mut self, index: RowIndex) -> anyhow::Result<IndexProgress> {
        let delta = self.index_until(Some(index.0), None)?;
        Ok(IndexProgress {
            row_count: self.row_count(),
            schema_delta: delta,
            bytes_scanned: self.scan_offset,
        })
    }

    fn scan_rows(
        &mut self,
        request: ScanRequest,
        visitor: &mut dyn RowVisitor,
    ) -> anyhow::Result<ScanProgress> {
        let mut current = request.start.0;
        let mut visited = 0;
        while visited < request.max_rows {
            let Some(row) = self.row(RowIndex(current))? else {
                break;
            };
            visited += 1;
            if visitor.visit(RowIndex(current), &row).is_break() {
                return Ok(ScanProgress {
                    visited,
                    next: None,
                    reached_end: false,
                });
            }
            match request.direction {
                ScanDirection::Forward => current += 1,
                ScanDirection::Reverse if current > 0 => current -= 1,
                ScanDirection::Reverse => {
                    return Ok(ScanProgress {
                        visited,
                        next: None,
                        reached_end: true,
                    });
                }
            }
        }
        let reached_end = self.eof && current >= self.offsets.len();
        Ok(ScanProgress {
            visited,
            next: (!reached_end).then_some(RowIndex(current)),
            reached_end,
        })
    }

    fn materialize(&mut self) -> anyhow::Result<InMemoryTable> {
        self.index_until(None, Some(u64::MAX))?;
        InMemoryTable::from_rows(
            self.generation,
            (0..self.offsets.len())
                .map(|index| {
                    self.typed_row(index)?
                        .ok_or_else(|| anyhow::anyhow!("indexed NDJSON row is unavailable"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        )
    }
}

fn typed_row_from_flat(
    generation: SourceGeneration,
    index: usize,
    raw: &FlatRow,
    schema: &JsonSchema,
) -> Row {
    let values = raw
        .cells
        .iter()
        .map(|(path, value, _)| (path.as_str(), value))
        .collect::<HashMap<_, _>>();
    let cells = schema
        .columns
        .iter()
        .map(|column| {
            let path = source_path(&column.source_identity);
            if path.is_empty() {
                match &column.source_identity {
                    ColumnSourceIdentity::Positional(index) => raw
                        .cells
                        .iter()
                        .find(|(_, _, identity)| {
                            identity == &ColumnSourceIdentity::Positional(*index)
                        })
                        .map(|(_, value, _)| value.clone())
                        .unwrap_or(CellValue::Null),
                    _ => CellValue::Null,
                }
            } else {
                values
                    .get(path)
                    .map(|value| (*value).clone())
                    .unwrap_or(CellValue::Null)
            }
        })
        .collect();
    Row::new(
        RowId {
            generation,
            ordinal: index as u64,
        },
        cells,
    )
}

#[derive(Debug, Clone)]
struct JsonTableStore {
    generation: SourceGeneration,
    rows: Vec<FlatRow>,
    schema: JsonSchema,
    indexed_rows: usize,
    bytes_scanned: u64,
    complete: bool,
}

impl JsonTableStore {
    fn typed_row(&self, index: usize) -> Option<Row> {
        let raw = self.rows.get(index)?;
        Some(typed_row_from_flat(
            self.generation,
            index,
            raw,
            &self.schema,
        ))
    }
}

impl TableStore for JsonTableStore {
    fn generation(&self) -> SourceGeneration {
        self.generation
    }

    fn row_count(&self) -> RowCount {
        RowCount::Exact(self.rows.len())
    }

    fn column_count(&self) -> usize {
        self.schema.columns.len()
    }

    fn row(&mut self, index: RowIndex) -> anyhow::Result<Option<Row>> {
        self.ensure_indexed_through(index)?;
        Ok(self.typed_row(index.0))
    }

    fn ensure_indexed_through(&mut self, index: RowIndex) -> anyhow::Result<IndexProgress> {
        let target = index.0.saturating_add(1).min(self.rows.len());
        let mut delta = SchemaDelta::default();
        while self.indexed_rows < target {
            let row = &self.rows[self.indexed_rows];
            let observed = self.schema.observe(row);
            delta.added_columns.extend(observed.added_columns);
            delta.widened_types.extend(observed.widened_types);
            self.bytes_scanned = self.bytes_scanned.saturating_add(row.source_bytes);
            self.indexed_rows += 1;
        }
        if self.indexed_rows == self.rows.len() && !self.complete {
            self.complete = true;
            delta.completed = true;
        }
        Ok(IndexProgress {
            row_count: self.row_count(),
            schema_delta: delta,
            bytes_scanned: self.bytes_scanned,
        })
    }

    fn scan_rows(
        &mut self,
        request: ScanRequest,
        visitor: &mut dyn RowVisitor,
    ) -> anyhow::Result<ScanProgress> {
        let mut current = request.start.0;
        let mut visited = 0;
        while visited < request.max_rows {
            let Some(row) = self.row(RowIndex(current))? else {
                break;
            };
            visited += 1;
            if visitor.visit(RowIndex(current), &row).is_break() {
                return Ok(ScanProgress {
                    visited,
                    next: None,
                    reached_end: false,
                });
            }
            match request.direction {
                ScanDirection::Forward => current += 1,
                ScanDirection::Reverse if current > 0 => current -= 1,
                ScanDirection::Reverse => {
                    return Ok(ScanProgress {
                        visited,
                        next: None,
                        reached_end: true,
                    });
                }
            }
        }
        let reached_end = current >= self.rows.len();
        Ok(ScanProgress {
            visited,
            next: (!reached_end).then_some(RowIndex(current)),
            reached_end,
        })
    }

    fn materialize(&mut self) -> anyhow::Result<InMemoryTable> {
        self.ensure_indexed_through(RowIndex(usize::MAX))?;
        InMemoryTable::from_rows(
            self.generation,
            (0..self.rows.len())
                .filter_map(|index| self.typed_row(index))
                .collect(),
        )
    }

    fn try_execute_query(&mut self, _query: &TableQuery) -> anyhow::Result<QueryExecution> {
        Ok(QueryExecution::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::LogicalType;

    fn open(input: &str, format: InputFormat, path: Option<&str>, scan_bytes: u64) -> OpenedTable {
        let pointer = path.map(|path| path.parse::<JsonPointer>().unwrap());
        let rows = match format {
            InputFormat::Json => parse_json_rows(input.as_bytes(), pointer.as_ref()).unwrap(),
            InputFormat::Ndjson => parse_ndjson_rows(input.as_bytes(), pointer.as_ref()).unwrap(),
            _ => unreachable!(),
        };
        open_json_rows(
            rows,
            "data".to_owned(),
            &OpenOptions {
                schema_scan_bytes: scan_bytes,
                ..OpenOptions::default()
            },
        )
        .unwrap()
        .into_implicit_table()
        .unwrap()
    }

    #[test]
    fn selects_pointer_flattens_objects_and_preserves_native_values() {
        let mut table = open(
            r#"{"took":1,"hits":{"hits":[{"_source":{"id":1,"ok":true,"empty":"","none":null,"tags":["a"]}}]}}"#,
            InputFormat::Json,
            Some("/hits/hits"),
            u64::MAX,
        );
        let identities = table
            .definition
            .columns
            .iter()
            .map(|column| source_path(&column.source_identity))
            .collect::<Vec<_>>();
        assert_eq!(
            identities,
            [
                "/_source/id",
                "/_source/ok",
                "/_source/empty",
                "/_source/none",
                "/_source/tags"
            ]
        );
        let row = table.store.row(RowIndex(0)).unwrap().unwrap();
        assert_eq!(row.cells[0], CellValue::Integer(1));
        assert_eq!(row.cells[1], CellValue::Boolean(true));
        assert_eq!(row.cells[2], CellValue::Text(String::new()));
        assert_eq!(row.cells[3], CellValue::Null);
        assert_eq!(row.cells[4], CellValue::Json("[\"a\"]".to_owned()));
    }

    #[test]
    fn ndjson_resolves_pointer_per_document_and_array_rows_are_positional() {
        let mut table = open(
            "{\"row\":[1,2]}\n{\"row\":[3,4]}\n",
            InputFormat::Ndjson,
            Some("/row"),
            u64::MAX,
        );
        assert!(matches!(
            table.definition.columns[0].source_identity,
            ColumnSourceIdentity::Positional(0)
        ));
        assert_eq!(
            table.store.row(RowIndex(1)).unwrap().unwrap().cells[1],
            CellValue::Integer(4)
        );
    }

    #[test]
    fn compact_labels_use_shortest_unique_suffix_and_escape_path_like_keys() {
        let table = open(
            r#"[{"customer":{"email":"a"},"billing":{"email":"b"},"a.b":1,"a/b":2}]"#,
            InputFormat::Json,
            None,
            u64::MAX,
        );
        let labels = table
            .definition
            .columns
            .iter()
            .map(|column| column.display_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels[0], "customer.email");
        assert_eq!(labels[1], "billing.email");
        assert!(labels[2].starts_with('['));
        assert!(labels[3].starts_with('['));
    }

    #[test]
    fn bounded_schema_is_provisional_and_late_columns_append_without_renaming() {
        let mut table = open(
            r#"[{"a":1},{"a":2,"late":true}]"#,
            InputFormat::Json,
            None,
            1,
        );
        assert_eq!(table.definition.schema_state, SchemaState::Provisional);
        assert_eq!(table.definition.columns.len(), 1);
        let original_label = table.definition.columns[0].display_name.clone();
        let progress = table.store.ensure_indexed_through(RowIndex(1)).unwrap();
        assert_eq!(progress.schema_delta.added_columns.len(), 1);
        table.definition.apply_delta(progress.schema_delta).unwrap();
        assert_eq!(table.definition.columns[0].display_name, original_label);
        assert_eq!(table.definition.columns[1].display_name, "late");
        assert_eq!(table.definition.schema_state, SchemaState::Complete);
        let first = table.store.row(RowIndex(0)).unwrap().unwrap();
        assert_eq!(first.cells[1], CellValue::Null);
    }

    #[test]
    fn generated_default_100_mib_boundary_is_bounded_and_full_scan_reaches_eof() {
        assert_eq!(crate::ingest::DEFAULT_SCHEMA_SCAN_BYTES, 100 * 1024 * 1024);
        let mut first = flatten_row(&serde_json::json!({"a": 1})).expect("first");
        first.source_bytes = crate::ingest::DEFAULT_SCHEMA_SCAN_BYTES + 1;
        let late = flatten_row(&serde_json::json!({"a": 2, "late": true})).expect("late");

        let mut bounded = open_json_rows(
            vec![first.clone(), late.clone()],
            "generated.json".to_owned(),
            &OpenOptions::default(),
        )
        .expect("bounded open")
        .into_implicit_table()
        .expect("table");
        assert_eq!(bounded.definition.schema_state, SchemaState::Provisional);
        assert_eq!(bounded.definition.columns.len(), 1);
        let stable_label = bounded.definition.columns[0].display_name.clone();
        let progress = bounded
            .store
            .ensure_indexed_through(RowIndex(1))
            .expect("late row");
        bounded
            .definition
            .apply_delta(progress.schema_delta)
            .expect("delta");
        assert_eq!(bounded.definition.columns.len(), 2);
        assert_eq!(bounded.definition.columns[0].display_name, stable_label);

        let full = open_json_rows(
            vec![first, late],
            "generated.json".to_owned(),
            &OpenOptions {
                schema_scan: SchemaScan::Full,
                ..OpenOptions::default()
            },
        )
        .expect("full open")
        .into_implicit_table()
        .expect("table");
        assert_eq!(full.definition.schema_state, SchemaState::Complete);
        assert_eq!(full.definition.columns.len(), 2);
        assert_eq!(full.store.row_count(), RowCount::Exact(2));
    }

    #[test]
    fn full_scan_widens_integer_to_float_and_marks_complete() {
        let rows = parse_json_rows(br#"[{"n":1},{"n":2.5}]"#, None).unwrap();
        let table = open_json_rows(
            rows,
            "data".to_owned(),
            &OpenOptions {
                schema_scan: SchemaScan::Full,
                schema_scan_bytes: 1,
                ..OpenOptions::default()
            },
        )
        .unwrap()
        .into_implicit_table()
        .unwrap();
        assert_eq!(table.definition.schema_state, SchemaState::Complete);
        assert_eq!(table.definition.columns[0].source_type, LogicalType::Float);
    }

    #[test]
    fn missing_and_scalar_pointer_selections_are_errors() {
        let root: Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
        assert!(resolve_pointer(&root, Some(&"/missing".parse().unwrap())).is_err());
        assert!(parse_json_rows(br#"{"a":1}"#, Some(&"/a".parse().unwrap())).is_err());
    }

    #[test]
    fn json_fixture_matrix_covers_supported_row_shapes_and_malformed_input() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample/json");
        for name in [
            "top-level-object.json",
            "array-of-objects.json",
            "array-of-arrays.json",
            "nested-values.json",
            "mixed-types.json",
            "path-like-keys.json",
        ] {
            let bytes = std::fs::read(root.join(name)).expect("fixture");
            assert!(!parse_json_rows(&bytes, None).expect(name).is_empty());
        }
        let malformed = std::fs::read(root.join("malformed.json")).expect("malformed");
        assert!(parse_json_rows(&malformed, None).is_err());
        let ndjson = std::fs::read(root.join("records.ndjson")).expect("ndjson");
        assert_eq!(parse_ndjson_rows(&ndjson, None).expect("records").len(), 2);
    }

    #[test]
    fn elasticsearch_fixture_exposes_only_row_relative_hit_columns() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("sample/json/elasticsearch-response.json"),
        )
        .expect("fixture");
        let rows = parse_json_rows(&bytes, Some(&"/hits/hits".parse().unwrap())).expect("hits");
        let table = open_json_rows(rows, "response.json".to_owned(), &OpenOptions::default())
            .unwrap()
            .into_implicit_table()
            .unwrap();
        let identities = table
            .definition
            .columns
            .iter()
            .map(|column| source_path(&column.source_identity))
            .collect::<Vec<_>>();
        assert!(identities.contains(&"/_source/user/id"));
        assert!(identities.contains(&"/_source/user/email"));
        assert!(!identities.iter().any(|path| path.contains("took")));
        assert!(!identities.iter().any(|path| path.contains("total")));
    }

    #[test]
    fn large_ndjson_indexes_document_offsets_and_decodes_random_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("records.ndjson");
        std::fs::write(
            &path,
            " \n{\"id\":1,\"text\":\"line\\nvalue\"}\n  {\"id\":2}\n{\"id\":3}\n",
        )
        .expect("write");
        let options = OpenOptions {
            format: InputFormat::Ndjson,
            lazy_threshold_bytes: 1,
            schema_scan_bytes: 1,
            ..OpenOptions::default()
        };
        let mut table = JsonAdapter::ndjson()
            .open(InputSource::Path(path), &options)
            .expect("open")
            .into_implicit_table()
            .expect("table");

        assert_eq!(table.definition.schema_state, SchemaState::Provisional);
        assert!(matches!(table.store.row_count(), RowCount::AtLeast(1)));
        assert_eq!(
            table.store.row(RowIndex(1)).unwrap().unwrap().cells[0],
            CellValue::Integer(2)
        );
        table
            .store
            .ensure_indexed_through(RowIndex(usize::MAX))
            .expect("finish indexing");
        assert_eq!(table.store.row_count(), RowCount::Exact(3));
        assert_eq!(
            table.store.row(RowIndex(0)).unwrap().unwrap().cells[1],
            CellValue::Text("line\nvalue".to_owned())
        );
    }

    #[test]
    fn large_ndjson_full_schema_scan_reaches_eof_without_retaining_flat_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("records.ndjson");
        std::fs::write(&path, "{\"a\":1}\n{\"late\":true}\n").expect("write");
        let options = OpenOptions {
            format: InputFormat::Ndjson,
            schema_scan: SchemaScan::Full,
            lazy_threshold_bytes: 1,
            schema_scan_bytes: 1,
            ..OpenOptions::default()
        };
        let mut table = JsonAdapter::ndjson()
            .open(InputSource::Path(path), &options)
            .expect("open")
            .into_implicit_table()
            .expect("table");

        assert_eq!(table.definition.schema_state, SchemaState::Complete);
        assert_eq!(table.store.row_count(), RowCount::Exact(2));
        assert_eq!(table.definition.columns.len(), 2);
        assert_eq!(
            table.store.row(RowIndex(0)).unwrap().unwrap().cells[1],
            CellValue::Null
        );
    }

    #[test]
    fn large_ndjson_detects_source_replacement_before_random_read() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("records.ndjson");
        std::fs::write(&path, "{\"id\":1}\n{\"id\":2}\n").expect("write");
        let options = OpenOptions {
            format: InputFormat::Ndjson,
            lazy_threshold_bytes: 1,
            schema_scan_bytes: 1,
            ..OpenOptions::default()
        };
        let mut table = JsonAdapter::ndjson()
            .open(InputSource::Path(path.clone()), &options)
            .expect("open")
            .into_implicit_table()
            .expect("table");
        let prior = table.store.row_count();

        std::fs::write(&path, "{\"changed\":true}\n").expect("replace");
        let error = table.store.row(RowIndex(1)).expect_err("changed source");
        assert!(error.to_string().contains("reload is required"));
        assert_eq!(table.store.row_count(), prior);
    }

    #[test]
    fn large_selected_json_array_indexes_nested_values_and_ignores_trailing_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("response.json");
        let padding = " ".repeat(8_193);
        std::fs::write(
            &path,
            format!(
                r#"{{"meta":{{"padding":"{padding}"}},"hits":{{"hits":[
                    {{"id":1,"text":"escaped \" quote","nested":{{"items":[1,{{"x":2}}]}}}},
                    {{"id":2,"text":"second"}},
                    {{"id":3,"late":true}}
                ]}},"tail":{{"must_be_ignored":[1,2,3]}}}}"#
            ),
        )
        .expect("write");
        let options = OpenOptions {
            format: InputFormat::Json,
            json_path: Some("/hits/hits".parse().unwrap()),
            lazy_threshold_bytes: 1,
            schema_scan_bytes: 1,
            ..OpenOptions::default()
        };
        let mut table = JsonAdapter::json()
            .open(InputSource::Path(path), &options)
            .expect("open")
            .into_implicit_table()
            .expect("table");

        assert_eq!(table.definition.schema_state, SchemaState::Provisional);
        assert!(matches!(table.store.row_count(), RowCount::AtLeast(1)));
        let progress = table
            .store
            .ensure_indexed_through(RowIndex(usize::MAX))
            .expect("finish");
        table.definition.apply_delta(progress.schema_delta).unwrap();
        assert_eq!(
            table.store.row(RowIndex(2)).unwrap().unwrap().cells[0],
            CellValue::Integer(3)
        );
        assert_eq!(table.store.row_count(), RowCount::Exact(3));
        assert!(table
            .definition
            .columns
            .iter()
            .any(|column| source_path(&column.source_identity) == "/late"));
        assert!(!table
            .definition
            .columns
            .iter()
            .any(|column| source_path(&column.source_identity).contains("tail")));
    }

    #[test]
    fn large_json_array_full_scan_reaches_eof_without_materialized_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("rows.json");
        std::fs::write(&path, "[ {\"a\":1},\n{\"b\":2}, {\"a\":3.5} ]").expect("write");
        let options = OpenOptions {
            format: InputFormat::Json,
            schema_scan: SchemaScan::Full,
            lazy_threshold_bytes: 1,
            schema_scan_bytes: 1,
            ..OpenOptions::default()
        };
        let mut table = JsonAdapter::json()
            .open(InputSource::Path(path), &options)
            .expect("open")
            .into_implicit_table()
            .expect("table");

        assert_eq!(table.definition.schema_state, SchemaState::Complete);
        assert_eq!(table.store.row_count(), RowCount::Exact(3));
        assert_eq!(table.definition.columns[0].source_type, LogicalType::Float);
        assert_eq!(
            table.store.row(RowIndex(1)).unwrap().unwrap().cells[0],
            CellValue::Null
        );
    }

    #[test]
    fn large_json_array_detects_mid_generation_replacement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("rows.json");
        std::fs::write(&path, "[{\"id\":1},{\"id\":2}]").expect("write");
        let options = OpenOptions {
            format: InputFormat::Json,
            lazy_threshold_bytes: 1,
            schema_scan_bytes: 1,
            ..OpenOptions::default()
        };
        let mut table = JsonAdapter::json()
            .open(InputSource::Path(path.clone()), &options)
            .expect("open")
            .into_implicit_table()
            .expect("table");
        let prior_count = table.store.row_count();

        std::fs::write(&path, "[{\"replacement\":true}]").expect("replace");
        let error = table
            .store
            .ensure_indexed_through(RowIndex(1))
            .expect_err("source change");
        assert!(error.to_string().contains("reload is required"));
        assert_eq!(table.store.row_count(), prior_count);
    }
}
