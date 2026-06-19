use crate::ops::sort::{
    infer_numeric_column_profile, is_numeric_cell, is_numeric_placeholder, NumericColumnProfile,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ColumnIndex(usize);

impl ColumnIndex {
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnMetadata {
    index: ColumnIndex,
    header: Option<String>,
    numeric_profile: NumericColumnProfile,
    numeric: bool,
}

impl ColumnMetadata {
    #[allow(dead_code, reason = "staged for upcoming column-oriented features")]
    pub(crate) fn index(&self) -> ColumnIndex {
        self.index
    }

    #[allow(dead_code, reason = "staged for upcoming column-oriented features")]
    pub(crate) fn header(&self) -> Option<&str> {
        self.header.as_deref()
    }

    pub(crate) fn numeric_profile(&self) -> NumericColumnProfile {
        self.numeric_profile
    }

    pub(crate) fn is_numeric(&self) -> bool {
        self.numeric
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Columns {
    metadata: Vec<ColumnMetadata>,
}

impl Columns {
    pub(crate) fn infer(header: Option<&[String]>, rows: &[Vec<String>]) -> Self {
        let column_count = header
            .map(<[String]>::len)
            .or_else(|| rows.first().map(Vec::len))
            .unwrap_or(0);
        let metadata = (0..column_count)
            .map(|column| {
                let header = header.and_then(|header| header.get(column)).cloned();
                let numeric_profile = infer_numeric_column_profile(header.as_deref(), rows, column);
                let numeric = is_numeric_column(rows, column, numeric_profile);
                ColumnMetadata {
                    index: ColumnIndex::new(column),
                    header,
                    numeric_profile,
                    numeric,
                }
            })
            .collect();
        Self { metadata }
    }

    pub(crate) fn len(&self) -> usize {
        self.metadata.len()
    }

    pub(crate) fn metadata(&self, column: ColumnIndex) -> Option<&ColumnMetadata> {
        self.metadata.get(column.as_usize())
    }

    pub(crate) fn numeric_profile(&self, column: ColumnIndex) -> NumericColumnProfile {
        self.metadata(column)
            .map(ColumnMetadata::numeric_profile)
            .unwrap_or_default()
    }

    pub(crate) fn is_numeric(&self, column: ColumnIndex) -> bool {
        self.metadata(column)
            .map(ColumnMetadata::is_numeric)
            .unwrap_or_default()
    }
}

fn is_numeric_column(rows: &[Vec<String>], column: usize, profile: NumericColumnProfile) -> bool {
    let mut has_numeric_value = false;
    for row in rows {
        let Some(cell) = row.get(column).map(|cell| cell.trim()) else {
            continue;
        };
        if cell.is_empty() || is_numeric_placeholder(cell) {
            continue;
        }
        if !is_numeric_cell(cell, profile) {
            return false;
        }
        has_numeric_value = true;
    }
    has_numeric_value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect()
    }

    #[test]
    fn infers_sticky_numeric_metadata_from_headers_and_rows() {
        let header = vec!["Name".to_owned(), "Duration".to_owned()];
        let rows = rows(&[&["alpha", "2m"], &["beta", "30"]]);
        let columns = Columns::infer(Some(&header), &rows);

        assert_eq!(columns.len(), 2);
        let metadata = columns
            .metadata(ColumnIndex::new(1))
            .expect("duration metadata");
        assert_eq!(metadata.index(), ColumnIndex::new(1));
        assert_eq!(metadata.header(), Some("Duration"));
        assert_eq!(
            columns.numeric_profile(ColumnIndex::new(1)),
            NumericColumnProfile::time()
        );
        assert!(columns.is_numeric(ColumnIndex::new(1)));
        assert!(!columns.is_numeric(ColumnIndex::new(0)));
    }
}
