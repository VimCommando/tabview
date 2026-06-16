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
    let positions = positions(rows);
    if positions.is_empty() {
        return None;
    }
    let start_idx = positions
        .iter()
        .position(|position| *position == start)
        .unwrap_or(0);

    let len = positions.len();
    let offsets = match direction {
        SearchDirection::Forward => Box::new((1..=len).map(|offset| (start_idx + offset) % len))
            as Box<dyn Iterator<Item = usize>>,
        SearchDirection::Reverse => {
            Box::new((1..=len).map(move |offset| (start_idx + len - offset) % len))
                as Box<dyn Iterator<Item = usize>>
        }
    };

    for idx in offsets {
        let position = positions[idx];
        if rows[position.row][position.column]
            .to_lowercase()
            .contains(&query)
        {
            return Some(position);
        }
    }
    None
}

fn positions(rows: &[Vec<String>]) -> Vec<Position> {
    rows.iter()
        .enumerate()
        .flat_map(|(row_idx, row)| {
            (0..row.len()).map(move |column_idx| Position {
                row: row_idx,
                column: column_idx,
            })
        })
        .collect()
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
}
