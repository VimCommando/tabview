use std::cmp::Ordering;

use regex::Regex;
use thiserror::Error;

use crate::ops::sort::{parse_numeric_scalar, NumericColumnProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterMode {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterKind {
    Text,
    Regex,
    Numeric,
}

impl FilterKind {
    pub(crate) fn all() -> [Self; 3] {
        [Self::Text, Self::Regex, Self::Numeric]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumericOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
}

#[derive(Debug, Clone)]
pub(crate) enum FilterCondition {
    Text(String),
    Regex(Regex),
    Numeric {
        operator: NumericOperator,
        operand: f64,
    },
}

impl FilterCondition {
    pub(crate) fn parse(
        kind: FilterKind,
        input: &str,
        profile: NumericColumnProfile,
    ) -> Result<Self, FilterParseError> {
        match kind {
            FilterKind::Text => Ok(Self::Text(input.to_owned())),
            FilterKind::Regex => Ok(Self::Regex(Regex::new(input)?)),
            FilterKind::Numeric => parse_numeric_condition(input, profile),
        }
    }

    pub(crate) fn matches(&self, value: &str, profile: NumericColumnProfile) -> bool {
        match self {
            Self::Text(needle) => value.contains(needle),
            Self::Regex(regex) => regex.is_match(value),
            Self::Numeric { operator, operand } => parse_numeric_scalar(value, profile)
                .is_some_and(|value| compare_numeric(value, *operator, *operand)),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ActiveFilter {
    pub(crate) column: usize,
    pub(crate) mode: FilterMode,
    #[allow(dead_code, reason = "retained for saved-view serialization")]
    pub(crate) kind: FilterKind,
    #[allow(dead_code, reason = "retained for saved-view serialization")]
    pub(crate) input: String,
    pub(crate) condition: FilterCondition,
}

impl ActiveFilter {
    pub(crate) fn new(
        column: usize,
        mode: FilterMode,
        kind: FilterKind,
        input: String,
        condition: FilterCondition,
    ) -> Self {
        Self {
            column,
            mode,
            kind,
            input,
            condition,
        }
    }

    #[allow(
        dead_code,
        reason = "kept for raw-only filter tests and fallback callers"
    )]
    pub(crate) fn accepts(&self, row: &[String], profile: NumericColumnProfile) -> bool {
        let value = row.get(self.column).map(String::as_str).unwrap_or_default();
        self.accepts_values(value, value, profile)
    }

    pub(crate) fn accepts_values(
        &self,
        raw: &str,
        rendered: &str,
        profile: NumericColumnProfile,
    ) -> bool {
        let matches = match &self.condition {
            FilterCondition::Text(_) | FilterCondition::Regex(_) => {
                self.condition.matches(raw, profile)
                    || (rendered != raw && self.condition.matches(rendered, profile))
            }
            FilterCondition::Numeric { .. } => self.condition.matches(raw, profile),
        };
        match self.mode {
            FilterMode::In => matches,
            FilterMode::Out => !matches,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum FilterParseError {
    #[error("invalid regex: {0}")]
    Regex(#[from] regex::Error),
    #[error("numeric filters need an operator: <, <=, >, >=, or =")]
    MissingNumericOperator,
    #[error("numeric filter operand is not recognized")]
    InvalidNumericOperand,
    #[error("numeric filters are not available for this column")]
    NumericUnavailable,
}

fn parse_numeric_condition(
    input: &str,
    profile: NumericColumnProfile,
) -> Result<FilterCondition, FilterParseError> {
    let input = input.trim();
    let (operator, operand) = if let Some(rest) = input.strip_prefix("<=") {
        (NumericOperator::LessThanOrEqual, rest)
    } else if let Some(rest) = input.strip_prefix(">=") {
        (NumericOperator::GreaterThanOrEqual, rest)
    } else if let Some(rest) = input.strip_prefix('<') {
        (NumericOperator::LessThan, rest)
    } else if let Some(rest) = input.strip_prefix('>') {
        (NumericOperator::GreaterThan, rest)
    } else if let Some(rest) = input.strip_prefix('=') {
        (NumericOperator::Equal, rest)
    } else {
        return Err(FilterParseError::MissingNumericOperator);
    };
    let operand = parse_numeric_scalar(operand.trim(), profile)
        .ok_or(FilterParseError::InvalidNumericOperand)?;
    Ok(FilterCondition::Numeric { operator, operand })
}

fn compare_numeric(value: f64, operator: NumericOperator, operand: f64) -> bool {
    match operator {
        NumericOperator::LessThan => value < operand,
        NumericOperator::LessThanOrEqual => value <= operand,
        NumericOperator::GreaterThan => value > operand,
        NumericOperator::GreaterThanOrEqual => value >= operand,
        NumericOperator::Equal => value.total_cmp(&operand) == Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_text_filter_matches_substrings() {
        let condition =
            FilterCondition::parse(FilterKind::Text, "foo", NumericColumnProfile::default())
                .expect("text condition");
        assert!(condition.matches("barfoobaz", NumericColumnProfile::default()));
        assert!(!condition.matches("bar", NumericColumnProfile::default()));
    }

    #[test]
    fn selected_regex_filter_matches_standard_regex() {
        let condition = FilterCondition::parse(
            FilterKind::Regex,
            "^foo[0-9]+$",
            NumericColumnProfile::default(),
        )
        .expect("regex condition");
        assert!(condition.matches("foo20", NumericColumnProfile::default()));
        assert!(!condition.matches("xfoo20", NumericColumnProfile::default()));
    }

    #[test]
    fn invalid_regex_is_reported() {
        assert!(matches!(
            FilterCondition::parse(FilterKind::Regex, "[", NumericColumnProfile::default()),
            Err(FilterParseError::Regex(_))
        ));
    }

    #[test]
    fn selected_numeric_filter_applies_operators() {
        let condition =
            FilterCondition::parse(FilterKind::Numeric, ">=20", NumericColumnProfile::default())
                .expect("numeric condition");
        assert!(condition.matches("20", NumericColumnProfile::default()));
        assert!(condition.matches("21", NumericColumnProfile::default()));
        assert!(!condition.matches("19", NumericColumnProfile::default()));
    }

    #[test]
    fn numeric_filter_supports_byte_suffixes() {
        let condition =
            FilterCondition::parse(FilterKind::Numeric, "<2gb", NumericColumnProfile::default())
                .expect("numeric condition");
        assert!(condition.matches("1500mb", NumericColumnProfile::default()));
        assert!(!condition.matches("3gb", NumericColumnProfile::default()));
    }

    #[test]
    fn numeric_filter_rejects_non_numeric_cells() {
        let condition =
            FilterCondition::parse(FilterKind::Numeric, "<10", NumericColumnProfile::default())
                .expect("numeric condition");
        assert!(!condition.matches("abc", NumericColumnProfile::default()));
    }

    #[test]
    fn active_filter_in_and_out_modes() {
        let condition =
            FilterCondition::parse(FilterKind::Text, "foo", NumericColumnProfile::default())
                .expect("text condition");
        let row = vec!["foobar".to_owned()];
        assert!(ActiveFilter::new(
            0,
            FilterMode::In,
            FilterKind::Text,
            "foo".to_owned(),
            condition.clone()
        )
        .accepts(&row, NumericColumnProfile::default()));
        assert!(!ActiveFilter::new(
            0,
            FilterMode::Out,
            FilterKind::Text,
            "foo".to_owned(),
            condition
        )
        .accepts(&row, NumericColumnProfile::default()));
    }
}
