use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Lexical,
    Natural,
    Numeric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

pub fn sort_rows(
    rows: &mut [Vec<String>],
    column: usize,
    mode: SortMode,
    direction: SortDirection,
) {
    rows.sort_by(|left, right| {
        let ordering = compare_cells(
            left.get(column).map(String::as_str).unwrap_or_default(),
            right.get(column).map(String::as_str).unwrap_or_default(),
            mode,
        );
        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

fn compare_cells(left: &str, right: &str, mode: SortMode) -> Ordering {
    match mode {
        SortMode::Lexical => left.cmp(right),
        SortMode::Natural => natural_tokens(left).cmp(&natural_tokens(right)),
        SortMode::Numeric => numeric_key(left).total_cmp(&numeric_key(right)),
    }
}

fn numeric_key(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(f64::INFINITY)
}

fn natural_tokens(value: &str) -> Vec<NaturalToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_digit = None;

    for ch in value.chars() {
        let is_digit = ch.is_ascii_digit();
        if current_is_digit == Some(is_digit) || current_is_digit.is_none() {
            current.push(ch);
            current_is_digit = Some(is_digit);
        } else {
            tokens.push(NaturalToken::from_part(
                &current,
                current_is_digit.unwrap_or(false),
            ));
            current.clear();
            current.push(ch);
            current_is_digit = Some(is_digit);
        }
    }

    if !current.is_empty() {
        tokens.push(NaturalToken::from_part(
            &current,
            current_is_digit.unwrap_or(false),
        ));
    }
    tokens
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum NaturalToken {
    Text(String),
    Number(u64),
}

impl NaturalToken {
    fn from_part(part: &str, is_digit: bool) -> Self {
        if is_digit {
            Self::Number(part.parse().unwrap_or(0))
        } else {
            Self::Text(part.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_lexically() {
        let mut rows = vec![vec!["b".to_owned()], vec!["a".to_owned()]];
        sort_rows(&mut rows, 0, SortMode::Lexical, SortDirection::Ascending);
        assert_eq!(rows, vec![vec!["a".to_owned()], vec!["b".to_owned()]]);
    }

    #[test]
    fn sorts_naturally() {
        let mut rows = vec![vec!["item10".to_owned()], vec!["item2".to_owned()]];
        sort_rows(&mut rows, 0, SortMode::Natural, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![vec!["item2".to_owned()], vec!["item10".to_owned()]]
        );
    }

    #[test]
    fn sorts_numerically() {
        let mut rows = vec![vec!["10".to_owned()], vec!["2".to_owned()]];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(rows, vec![vec!["2".to_owned()], vec!["10".to_owned()]]);
    }
}
