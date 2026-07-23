use crate::table::{
    ColumnDefinition, ColumnId, ColumnSourceIdentity, InMemoryTable, LazyFileTable, LogicalType,
    OffsetTableStore, RelationMetadata, RowIndex, SchemaState, SourceGeneration, TableDefinition,
    TableStore, TypeOrigin,
};

use super::adapter::{OpenedSource, OpenedTable, ProbeResult, SourceAdapter};
use super::source::{read_source, InputSource};
use super::{parse_rows, InputFormat, OpenOptions};

#[derive(Debug, Default)]
pub struct DelimitedAdapter;

impl SourceAdapter for DelimitedAdapter {
    fn format(&self) -> InputFormat {
        InputFormat::Delimited
    }

    fn probe(&self, _source: &InputSource, sample: &[u8]) -> ProbeResult {
        let Ok(text) = std::str::from_utf8(sample) else {
            return ProbeResult::Possible;
        };
        if super::sniff_delimiter(text).is_some() {
            ProbeResult::Strong
        } else {
            ProbeResult::Possible
        }
    }

    fn open(&self, source: InputSource, options: &OpenOptions) -> anyhow::Result<OpenedSource> {
        options.validate()?;
        if let InputSource::Path(path) = &source {
            if std::fs::metadata(path)?.len() >= options.lazy_threshold_bytes
                && LazyFileTable::supports_options(&options.delimited)
            {
                let mut store = LazyFileTable::open(path, options.delimited.clone())?;
                let generation = store.generation();
                let mut sample_rows = Vec::new();
                for index in 0..2 {
                    if let Some(row) = store.row(RowIndex(index))? {
                        sample_rows.push(row.display_cells());
                    }
                }
                let (definition, header_rows) =
                    delimited_definition(generation, &sample_rows, source.display_name());
                let store: Box<dyn TableStore> = if header_rows == 0 {
                    Box::new(store)
                } else {
                    Box::new(OffsetTableStore::new(Box::new(store), header_rows))
                };
                return Ok(OpenedSource::implicit(OpenedTable {
                    generation,
                    definition,
                    store,
                    object_mode: None,
                    warnings: Vec::new(),
                }));
            }
        }
        let bytes = read_source(&source)?;
        let rows = parse_rows(&bytes, &options.delimited)?;
        let display_name = source.display_name();
        Ok(OpenedSource::implicit(open_delimited_rows(
            rows,
            display_name,
        )))
    }
}

pub fn open_delimited_rows(rows: Vec<Vec<String>>, display_name: String) -> OpenedTable {
    let generation = SourceGeneration::new();
    let (definition, header_rows) = delimited_definition(generation, &rows, display_name);
    let data_rows = rows.into_iter().skip(header_rows).collect();
    OpenedTable {
        generation,
        definition,
        store: Box::new(InMemoryTable::from_text_rows(generation, data_rows)),
        object_mode: None,
        warnings: Vec::new(),
    }
}

fn delimited_definition(
    generation: SourceGeneration,
    rows: &[Vec<String>],
    display_name: String,
) -> (TableDefinition, usize) {
    let has_header = rows.len() > 1
        && rows
            .first()
            .is_some_and(|row| !row.iter().any(|cell| cell.parse::<f64>().is_ok()));
    let (header, data_rows) = if has_header {
        (rows.first(), &rows[1..])
    } else {
        (None, rows)
    };
    let column_count = header
        .as_ref()
        .map(|header| header.len())
        .unwrap_or_default()
        .max(data_rows.iter().map(Vec::len).max().unwrap_or_default());
    let columns = (0..column_count)
        .map(|ordinal| {
            let name = header
                .as_ref()
                .and_then(|header| header.get(ordinal))
                .cloned();
            let display_name = name
                .as_deref()
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("Column {}", ordinal + 1));
            ColumnDefinition {
                id: ColumnId {
                    generation,
                    ordinal: ordinal as u32,
                },
                source_identity: ColumnSourceIdentity::Delimited {
                    ordinal,
                    name: name.clone(),
                },
                display_name,
                source_type: LogicalType::Text,
                type_origin: TypeOrigin::Declared,
            }
        })
        .collect();
    let relation = RelationMetadata::implicit(display_name, has_header);
    (
        TableDefinition {
            generation,
            columns,
            schema_state: SchemaState::Complete,
            relation,
        },
        usize::from(has_header),
    )
}

#[cfg(test)]
mod tests {
    use crate::table::{CellValue, RowCount, RowIndex};

    use super::*;

    #[test]
    fn moves_header_classification_into_table_definition() {
        let mut opened = open_delimited_rows(
            vec![
                vec!["name".to_owned(), "name".to_owned(), String::new()],
                vec!["a".to_owned(), "b".to_owned(), String::new()],
            ],
            "data.csv".to_owned(),
        );
        assert!(opened.definition.relation.header_visible);
        assert_eq!(opened.definition.columns[0].display_name, "name");
        assert_eq!(opened.definition.columns[1].display_name, "name");
        assert_ne!(
            opened.definition.columns[0].id,
            opened.definition.columns[1].id
        );
        assert_eq!(opened.definition.columns[2].display_name, "Column 3");
        assert_eq!(opened.store.row_count(), RowCount::Exact(1));
        assert_eq!(
            opened
                .store
                .row(RowIndex(0))
                .expect("read")
                .expect("row")
                .cells,
            vec![
                CellValue::Text("a".to_owned()),
                CellValue::Text("b".to_owned()),
                CellValue::Text(String::new())
            ]
        );
    }

    #[test]
    fn headerless_columns_have_stable_generated_definitions() {
        let opened = open_delimited_rows(
            vec![
                vec!["1".to_owned(), "2".to_owned()],
                vec!["3".to_owned(), "4".to_owned()],
            ],
            "numbers.csv".to_owned(),
        );
        assert!(!opened.definition.relation.header_visible);
        assert_eq!(opened.definition.columns[0].display_name, "Column 1");
        assert!(matches!(
            opened.definition.columns[0].source_identity,
            ColumnSourceIdentity::Delimited {
                ordinal: 0,
                name: None
            }
        ));
    }

    #[test]
    fn large_seekable_input_uses_incremental_store_and_skips_header() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.csv");
        std::fs::write(&path, "name,value\na,1\nb,2\n").expect("write");
        let options = OpenOptions {
            lazy_threshold_bytes: 0,
            ..OpenOptions::default()
        };
        let source = DelimitedAdapter
            .open(InputSource::Path(path), &options)
            .expect("open");
        let mut table = source.into_implicit_table().expect("table");
        assert!(table.definition.relation.header_visible);
        assert_eq!(table.definition.columns[0].display_name, "name");
        assert_eq!(table.store.row_count(), RowCount::AtLeast(1));
        assert_eq!(
            table
                .store
                .row(RowIndex(0))
                .expect("read")
                .expect("row")
                .display_cells(),
            ["a", "1"]
        );
    }

    #[test]
    fn unsafe_byte_encoding_falls_back_to_materialized_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.tsv");
        let mut bytes = vec![0xff, 0xfe];
        for unit in "a\tb\n1\t2\n".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, bytes).expect("write");
        let options = OpenOptions {
            lazy_threshold_bytes: 0,
            delimited: crate::ingest::ParseOptions {
                encoding: Some("utf-16".to_owned()),
                ..crate::ingest::ParseOptions::default()
            },
            ..OpenOptions::default()
        };
        let source = DelimitedAdapter
            .open(InputSource::Path(path), &options)
            .expect("open");
        let table = source.into_implicit_table().expect("table");
        assert_eq!(table.store.row_count(), RowCount::Exact(1));
    }
}
