use super::{ColumnId, SourceGeneration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullPlacement {
    First,
    Last,
}

impl Default for NullPlacement {
    fn default() -> Self {
        Self::Last
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDomain {
    Raw,
    Rendered,
    RawOrRendered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    In,
    Out,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterPredicate {
    Text {
        value: String,
        domain: ValueDomain,
    },
    Regex {
        pattern: String,
        domain: ValueDomain,
    },
    Numeric {
        operator: NumericOperator,
        operand: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterSpec {
    pub column: ColumnId,
    pub mode: FilterMode,
    pub predicate: FilterPredicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Lexical,
    Natural,
    Numeric,
    Date,
    SemanticVersion,
    Ip,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortSpec {
    pub column: ColumnId,
    pub mode: SortMode,
    pub direction: SortDirection,
    pub nulls: NullPlacement,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableQuery {
    pub generation: SourceGeneration,
    pub filters: Vec<FilterSpec>,
    pub order_by: Vec<SortSpec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_references_generation_scoped_columns_and_resolved_nulls() {
        let generation = SourceGeneration::new();
        let column = ColumnId {
            generation,
            ordinal: 1,
        };
        let query = TableQuery {
            generation,
            filters: vec![FilterSpec {
                column,
                mode: FilterMode::In,
                predicate: FilterPredicate::Text {
                    value: "ok".to_owned(),
                    domain: ValueDomain::RawOrRendered,
                },
            }],
            order_by: vec![SortSpec {
                column,
                mode: SortMode::Natural,
                direction: SortDirection::Descending,
                nulls: NullPlacement::First,
            }],
        };
        assert_eq!(query.order_by[0].nulls, NullPlacement::First);
        assert_eq!(query.filters[0].column.generation, generation);
    }
}
