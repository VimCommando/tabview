use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;

use crate::ingest::{decode_input, parse_rows, sniff_delimiter, ParseOptions, Quoting};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    cells: Vec<String>,
}

impl Row {
    pub fn new(cells: Vec<String>) -> Self {
        Self { cells }
    }

    pub fn cells(&self) -> &[String] {
        &self.cells
    }
}

pub trait TableStore {
    fn row_count(&self) -> Option<usize>;
    fn column_count(&self) -> usize;
    fn materialize(&mut self) -> anyhow::Result<Vec<Vec<String>>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryTable {
    rows: Vec<Row>,
    column_count: usize,
}

impl InMemoryTable {
    pub fn new(rows: Vec<Vec<String>>) -> Self {
        let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
        Self {
            rows: rows.into_iter().map(Row::new).collect(),
            column_count,
        }
    }

    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }
}

impl TableStore for InMemoryTable {
    fn row_count(&self) -> Option<usize> {
        Some(self.rows.len())
    }

    fn column_count(&self) -> usize {
        self.column_count
    }

    fn materialize(&mut self) -> anyhow::Result<Vec<Vec<String>>> {
        Ok(self.rows.iter().map(|row| row.cells.clone()).collect())
    }
}

#[derive(Debug, Clone)]
pub struct LazyFileTable {
    path: PathBuf,
    offsets: Vec<u64>,
    options: ParseOptions,
    column_count: usize,
}

impl LazyFileTable {
    pub fn open(path: impl AsRef<Path>, options: ParseOptions) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut options = options;
        if options.delimiter.is_none() {
            options.delimiter = Some(sniff_file_delimiter(&path)?);
        }

        let mut reader = csv_reader_builder(&options).from_reader(File::open(&path)?);
        let mut offsets = Vec::new();
        let mut record = csv::ByteRecord::new();
        let mut column_count = 0;

        while reader.read_byte_record(&mut record)? {
            if let Some(position) = record.position() {
                offsets.push(position.byte());
            }
            column_count = column_count.max(record.len());
        }

        Ok(Self {
            path,
            offsets,
            options,
            column_count,
        })
    }

    pub fn row(&self, index: usize) -> anyhow::Result<Option<Row>> {
        let Some(offset) = self.offsets.get(index).copied() else {
            return Ok(None);
        };
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut reader = csv_reader_builder(&self.options).from_reader(file);
        let mut record = csv::ByteRecord::new();
        if !reader.read_byte_record(&mut record)? {
            return Ok(None);
        }
        Ok(Some(row_from_byte_record(&record, &self.options)?))
    }
}

impl TableStore for LazyFileTable {
    fn row_count(&self) -> Option<usize> {
        Some(self.offsets.len())
    }

    fn column_count(&self) -> usize {
        self.column_count
    }

    fn materialize(&mut self) -> anyhow::Result<Vec<Vec<String>>> {
        let data = std::fs::read(&self.path)?;
        Ok(parse_rows(&data, &self.options)?)
    }
}

fn sniff_file_delimiter(path: &Path) -> anyhow::Result<u8> {
    let mut sample = Vec::new();
    File::open(path)?.take(64 * 1024).read_to_end(&mut sample)?;
    let sample = String::from_utf8_lossy(&sample);
    Ok(sniff_delimiter(&sample).unwrap_or(b','))
}

fn row_from_byte_record(record: &csv::ByteRecord, options: &ParseOptions) -> anyhow::Result<Row> {
    let cells = record
        .iter()
        .map(|field| decode_input(field, options.encoding.as_deref()).map(|decoded| decoded.text))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Row::new(cells))
}

fn csv_reader_builder(options: &ParseOptions) -> ReaderBuilder {
    let mut builder = ReaderBuilder::new();
    builder
        .has_headers(false)
        .flexible(true)
        .delimiter(options.delimiter.unwrap_or(b','))
        .quote(options.quote_char);
    if options.quoting == Some(Quoting::None) {
        builder.quoting(false);
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_rectangular_rows() {
        let table = InMemoryTable::new(vec![vec!["a".to_owned(), "b".to_owned()]]);
        assert_eq!(table.row_count(), Some(1));
        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row(0).expect("row").cells(), ["a", "b"]);
    }

    #[test]
    fn in_memory_table_tracks_max_column_count() {
        let table = InMemoryTable::new(vec![
            vec!["a".to_owned()],
            vec!["1".to_owned(), "2".to_owned(), "3".to_owned()],
        ]);

        assert_eq!(table.row_count(), Some(2));
        assert_eq!(table.column_count(), 3);
    }

    #[test]
    fn lazy_file_table_reads_rows_by_offset_and_materializes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.csv");
        std::fs::write(&path, "a,b\n1,2\n").expect("write");
        let mut table = LazyFileTable::open(&path, ParseOptions::default()).expect("lazy table");

        assert_eq!(table.row_count(), Some(2));
        assert_eq!(table.column_count(), 2);
        assert_eq!(table.row(1).expect("row").expect("row").cells(), ["1", "2"]);
        assert_eq!(
            table.materialize().expect("materialize"),
            vec![
                vec!["a".to_owned(), "b".to_owned()],
                vec!["1".to_owned(), "2".to_owned()]
            ]
        );
    }

    #[test]
    fn lazy_file_table_indexes_multiline_csv_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.csv");
        std::fs::write(&path, "a,b\n\"hello\nworld\",2\nx,y\n").expect("write");
        let mut table = LazyFileTable::open(&path, ParseOptions::default()).expect("lazy table");

        assert_eq!(table.row_count(), Some(3));
        assert_eq!(table.column_count(), 2);
        assert_eq!(
            table.row(1).expect("row").expect("row").cells(),
            ["hello\nworld", "2"]
        );
        assert_eq!(
            table.materialize().expect("materialize"),
            vec![
                vec!["a".to_owned(), "b".to_owned()],
                vec!["hello\nworld".to_owned(), "2".to_owned()],
                vec!["x".to_owned(), "y".to_owned()]
            ]
        );
    }

    #[test]
    fn lazy_file_table_tracks_max_column_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ragged.csv");
        std::fs::write(&path, "a\n1,2,3\n").expect("write");
        let table = LazyFileTable::open(&path, ParseOptions::default()).expect("lazy table");

        assert_eq!(table.row_count(), Some(2));
        assert_eq!(table.column_count(), 3);
    }

    #[test]
    fn lazy_file_table_materializes_non_utf8_input_with_parse_options() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("latin1.csv");
        std::fs::write(&path, b"caf\xe9,2\n").expect("write");
        let mut table = LazyFileTable::open(
            &path,
            ParseOptions {
                encoding: Some("latin-1".to_owned()),
                ..ParseOptions::default()
            },
        )
        .expect("lazy table");

        assert_eq!(
            table.row(0).expect("row").expect("row").cells(),
            ["café", "2"]
        );
        assert_eq!(
            table.materialize().expect("materialize"),
            vec![vec!["café".to_owned(), "2".to_owned()]]
        );
    }
}
