use std::ops::Range;

use regex::{Regex, RegexBuilder};

use crate::view::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Reverse,
}

#[derive(Debug, Clone)]
pub(crate) struct CaseInsensitiveQuery<'a> {
    raw: &'a str,
    matcher: Regex,
}

impl<'a> CaseInsensitiveQuery<'a> {
    pub(crate) fn new(raw: &'a str) -> Option<Self> {
        if raw.is_empty() {
            return None;
        }
        RegexBuilder::new(&regex::escape(raw))
            .case_insensitive(true)
            .build()
            .ok()
            .map(|matcher| Self { raw, matcher })
    }

    pub(crate) fn matches(&self, value: &str) -> bool {
        self.find(value).is_some()
    }

    pub(crate) fn find(&self, value: &str) -> Option<Range<usize>> {
        if value.is_ascii() && self.raw.is_ascii() {
            return value
                .as_bytes()
                .windows(self.raw.len())
                .position(|window| window.eq_ignore_ascii_case(self.raw.as_bytes()))
                .map(|start| start..start + self.raw.len());
        }
        self.matcher
            .find(value)
            .map(|matched| matched.start()..matched.end())
    }

    pub(crate) fn find_iter<'b>(&'b self, value: &'b str) -> MatchRanges<'a, 'b> {
        MatchRanges {
            query: self,
            value,
            offset: 0,
        }
    }
}

pub(crate) struct MatchRanges<'a, 'b> {
    query: &'b CaseInsensitiveQuery<'a>,
    value: &'b str,
    offset: usize,
}

impl Iterator for MatchRanges<'_, '_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let range = self.query.find(self.value.get(self.offset..)?)?;
        let start = self.offset + range.start;
        let end = self.offset + range.end;
        self.offset = end.max(start + 1);
        Some(start..end)
    }
}

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn query_finds_ascii_case_insensitive_ranges() {
        let query = CaseInsensitiveQuery::new("AL").expect("query");
        assert_eq!(query.find("alpha"), Some(0..2));
    }

    #[test]
    fn query_iterates_non_overlapping_ranges() {
        let query = CaseInsensitiveQuery::new("a").expect("query");
        assert_eq!(
            query.find_iter("banana").collect::<Vec<_>>(),
            vec![1..2, 3..4, 5..6]
        );
    }

    #[test]
    fn query_finds_unicode_case_insensitive_ranges() {
        let query = CaseInsensitiveQuery::new("CAFÉ").expect("query");
        assert_eq!(query.find("xx café yy"), Some(3..8));
    }
}

pub fn find_match(
    rows: &[Vec<String>],
    start: Position,
    query: &str,
    direction: SearchDirection,
) -> Option<Position> {
    if rows.is_empty() {
        return None;
    }
    let query = CaseInsensitiveQuery::new(query)?;
    let Some(mut position) = start_or_virtual_wrap_position(rows, start, direction) else {
        return None;
    };

    for _ in 0..cell_count(rows) {
        position = next_position(rows, position, direction)?;
        if query.matches(&rows[position.row][position.column]) {
            return Some(position);
        }
    }
    None
}

fn start_or_virtual_wrap_position(
    rows: &[Vec<String>],
    start: Position,
    direction: SearchDirection,
) -> Option<Position> {
    rows.get(start.row)
        .and_then(|row| (start.column < row.len()).then_some(start))
        .or_else(|| match direction {
            SearchDirection::Forward => last_position(rows),
            SearchDirection::Reverse => first_position(rows),
        })
}

fn first_position(rows: &[Vec<String>]) -> Option<Position> {
    rows.iter()
        .enumerate()
        .find(|(_, row)| !row.is_empty())
        .map(|(row, _)| Position { row, column: 0 })
}

fn last_position(rows: &[Vec<String>]) -> Option<Position> {
    rows.iter()
        .enumerate()
        .rev()
        .find(|(_, row)| !row.is_empty())
        .map(|(row, row_values)| Position {
            row,
            column: row_values.len() - 1,
        })
}

fn next_position(
    rows: &[Vec<String>],
    position: Position,
    direction: SearchDirection,
) -> Option<Position> {
    match direction {
        SearchDirection::Forward => next_forward_position(rows, position),
        SearchDirection::Reverse => next_reverse_position(rows, position),
    }
}

fn next_forward_position(rows: &[Vec<String>], position: Position) -> Option<Position> {
    if rows[position.row].get(position.column + 1).is_some() {
        return Some(Position {
            row: position.row,
            column: position.column + 1,
        });
    }

    rows.iter()
        .enumerate()
        .skip(position.row + 1)
        .find(|(_, row)| !row.is_empty())
        .map(|(row, _)| Position { row, column: 0 })
        .or_else(|| first_position(rows))
}

fn next_reverse_position(rows: &[Vec<String>], position: Position) -> Option<Position> {
    if position.column > 0 {
        return Some(Position {
            row: position.row,
            column: position.column - 1,
        });
    }

    rows.iter()
        .enumerate()
        .take(position.row)
        .rev()
        .find(|(_, row)| !row.is_empty())
        .map(|(row, row_values)| Position {
            row,
            column: row_values.len() - 1,
        })
        .or_else(|| last_position(rows))
}

fn cell_count(rows: &[Vec<String>]) -> usize {
    rows.iter().map(Vec::len).sum()
}

#[cfg(test)]
fn contains_case_insensitive(value: &str, query: &str) -> bool {
    query.is_empty() || CaseInsensitiveQuery::new(query).is_some_and(|query| query.matches(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Vec<String>> {
        vec![
            vec!["alpha".to_owned(), "beta".to_owned()],
            vec!["gamma".to_owned(), "delta".to_owned()],
        ]
    }

    #[test]
    fn finds_next_match_with_wraparound() {
        assert_eq!(
            find_match(
                &rows(),
                Position { row: 1, column: 1 },
                "alpha",
                SearchDirection::Forward,
            ),
            Some(Position { row: 0, column: 0 })
        );
    }

    #[test]
    fn finds_previous_match_without_mutating_rows() {
        let rows = rows();
        let original = rows.clone();
        assert_eq!(
            find_match(
                &rows,
                Position { row: 0, column: 0 },
                "delta",
                SearchDirection::Reverse,
            ),
            Some(Position { row: 1, column: 1 })
        );
        assert_eq!(rows, original);
    }

    #[test]
    fn skips_empty_rows_without_allocating_position_list() {
        let rows = vec![
            vec!["alpha".to_owned()],
            Vec::new(),
            vec!["beta".to_owned()],
        ];

        assert_eq!(
            find_match(
                &rows,
                Position { row: 0, column: 0 },
                "BETA",
                SearchDirection::Forward,
            ),
            Some(Position { row: 2, column: 0 })
        );
    }

    #[test]
    fn invalid_start_checks_edge_cell_first() {
        let rows = rows();

        assert_eq!(
            find_match(
                &rows,
                Position {
                    row: usize::MAX,
                    column: usize::MAX,
                },
                "alpha",
                SearchDirection::Forward,
            ),
            Some(Position { row: 0, column: 0 })
        );
        assert_eq!(
            find_match(
                &rows,
                Position {
                    row: usize::MAX,
                    column: usize::MAX,
                },
                "delta",
                SearchDirection::Reverse,
            ),
            Some(Position { row: 1, column: 1 })
        );
    }

    #[test]
    fn contains_case_insensitive_handles_empty_query() {
        assert!(contains_case_insensitive("alpha", ""));
    }

    #[test]
    fn ascii_uppercase_query_matches_non_ascii_value() {
        let query = CaseInsensitiveQuery::new("CAF").expect("query");

        assert!(query.matches("café"));
    }
}
