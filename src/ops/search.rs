use crate::view::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Reverse,
}

pub fn find_match(
    rows: &[Vec<String>],
    start: Position,
    query: &str,
    direction: SearchDirection,
) -> Option<Position> {
    if query.is_empty() || rows.is_empty() {
        return None;
    }
    let query = query.to_lowercase();
    let Some(mut position) = start_or_virtual_wrap_position(rows, start, direction) else {
        return None;
    };

    for _ in 0..cell_count(rows) {
        position = next_position(rows, position, direction)?;
        if contains_case_insensitive_folded_query(&rows[position.row][position.column], &query) {
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

pub(crate) fn contains_case_insensitive(value: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    if value.is_ascii() && query.is_ascii() {
        return value
            .as_bytes()
            .windows(query.len())
            .any(|window| window.eq_ignore_ascii_case(query.as_bytes()));
    }

    value.to_lowercase().contains(&query.to_lowercase())
}

fn contains_case_insensitive_folded_query(value: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    if value.is_ascii() && query.is_ascii() {
        return value
            .as_bytes()
            .windows(query.len())
            .any(|window| window.eq_ignore_ascii_case(query.as_bytes()));
    }

    value.to_lowercase().contains(query)
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
}
