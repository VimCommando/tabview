use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ingest::JsonPointer;

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceGeneration(pub u64);

impl SourceGeneration {
    pub fn new() -> Self {
        Self(NEXT_GENERATION.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for SourceGeneration {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowId {
    pub generation: SourceGeneration,
    pub ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnId {
    pub generation: SourceGeneration,
    pub ordinal: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Binary(Vec<u8>),
    Json(String),
}

impl CellValue {
    pub fn display(&self) -> Cow<'_, str> {
        match self {
            Self::Null => Cow::Borrowed(""),
            Self::Boolean(value) => Cow::Borrowed(if *value { "true" } else { "false" }),
            Self::Integer(value) => Cow::Owned(value.to_string()),
            Self::Float(value) => Cow::Owned(value.to_string()),
            Self::Text(value) | Self::Json(value) => Cow::Borrowed(value),
            Self::Binary(value) => String::from_utf8_lossy(value),
        }
    }

    pub fn logical_type(&self) -> LogicalType {
        match self {
            Self::Null => LogicalType::Null,
            Self::Boolean(_) => LogicalType::Boolean,
            Self::Integer(_) => LogicalType::Integer,
            Self::Float(_) => LogicalType::Float,
            Self::Text(_) => LogicalType::Text,
            Self::Binary(_) => LogicalType::Binary,
            Self::Json(_) => LogicalType::Structured,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: RowId,
    pub cells: Vec<CellValue>,
}

impl Row {
    pub fn new(id: RowId, cells: Vec<CellValue>) -> Self {
        Self { id, cells }
    }

    pub fn from_text(generation: SourceGeneration, ordinal: usize, cells: Vec<String>) -> Self {
        Self::new(
            RowId {
                generation,
                ordinal: ordinal as u64,
            },
            cells.into_iter().map(CellValue::Text).collect(),
        )
    }

    pub fn display_cells(&self) -> Vec<String> {
        self.cells
            .iter()
            .map(|cell| cell.display().into_owned())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogicalType {
    #[default]
    Unknown,
    Null,
    Boolean,
    Integer,
    Float,
    Text,
    Binary,
    Structured,
    Mixed,
}

impl LogicalType {
    pub fn widen(self, next: Self) -> Self {
        use LogicalType::*;
        match (self, next) {
            (Unknown | Null, value) | (value, Unknown | Null) => value,
            (Integer, Float) | (Float, Integer) => Float,
            (left, right) if left == right => left,
            _ => Mixed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOrigin {
    Declared,
    Inferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColumnSourceIdentity {
    Delimited {
        ordinal: usize,
        name: Option<String>,
    },
    JsonPointer(JsonPointer),
    Positional(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDefinition {
    pub id: ColumnId,
    pub source_identity: ColumnSourceIdentity,
    pub display_name: String,
    pub source_type: LogicalType,
    pub type_origin: TypeOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaState {
    Provisional,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeWidening {
    pub column: ColumnId,
    pub source_type: LogicalType,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaDelta {
    pub added_columns: Vec<ColumnDefinition>,
    pub widened_types: Vec<TypeWidening>,
    pub completed: bool,
}

impl SchemaDelta {
    pub fn is_empty(&self) -> bool {
        self.added_columns.is_empty() && self.widened_types.is_empty() && !self.completed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationMetadata {
    pub name: String,
    pub display_name: String,
    pub header_visible: bool,
}

impl RelationMetadata {
    pub fn implicit(display_name: impl Into<String>, header_visible: bool) -> Self {
        Self {
            name: "table".to_owned(),
            display_name: display_name.into(),
            header_visible,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDefinition {
    pub generation: SourceGeneration,
    pub columns: Vec<ColumnDefinition>,
    pub schema_state: SchemaState,
    pub relation: RelationMetadata,
}

impl TableDefinition {
    pub fn apply_delta(&mut self, delta: SchemaDelta) -> anyhow::Result<()> {
        for column in &delta.added_columns {
            if column.id.generation != self.generation
                || self.columns.iter().any(|current| current.id == column.id)
            {
                anyhow::bail!("schema delta is not append-only for the active generation");
            }
        }
        self.columns.extend(delta.added_columns);
        for widening in delta.widened_types {
            let Some(column) = self
                .columns
                .iter_mut()
                .find(|column| column.id == widening.column)
            else {
                anyhow::bail!("schema delta widens an unknown column");
            };
            column.source_type = column.source_type.widen(widening.source_type);
        }
        if delta.completed {
            self.schema_state = SchemaState::Complete;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_values_keep_null_empty_and_native_kinds_distinct() {
        let values = vec![
            CellValue::Null,
            CellValue::Text(String::new()),
            CellValue::Boolean(true),
            CellValue::Integer(2),
            CellValue::Float(2.5),
            CellValue::Text("2".to_owned()),
            CellValue::Binary(b"bin".to_vec()),
            CellValue::Json("[1,2]".to_owned()),
        ];
        assert_ne!(values[0], values[1]);
        assert_eq!(values[0].display(), "");
        assert_eq!(values[1].display(), "");
        assert_eq!(values[2].display(), "true");
        assert_eq!(values[3].logical_type(), LogicalType::Integer);
        assert_eq!(values[4].logical_type(), LogicalType::Float);
        assert_eq!(values[5].logical_type(), LogicalType::Text);
        assert_eq!(values[6].logical_type(), LogicalType::Binary);
        assert_eq!(values[7].logical_type(), LogicalType::Structured);
    }

    #[test]
    fn source_generations_scope_row_and_column_ids() {
        let first = SourceGeneration::new();
        let second = SourceGeneration::new();
        assert_ne!(first, second);
        assert_ne!(
            RowId {
                generation: first,
                ordinal: 0
            },
            RowId {
                generation: second,
                ordinal: 0
            }
        );
    }
}
