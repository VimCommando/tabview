use crate::view::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

pub fn skip_to_change(
    rows: &[Vec<String>],
    start: Position,
    axis: Axis,
    direction: Direction,
    count: usize,
) -> Position {
    let mut position = start;
    for _ in 0..count.max(1) {
        position = skip_once(rows, position, axis, direction);
    }
    position
}

fn skip_once(rows: &[Vec<String>], start: Position, axis: Axis, direction: Direction) -> Position {
    let Some(start_value) = cell(rows, start) else {
        return start;
    };
    let mut current = step(start, axis, direction);
    while let Some(position) = current {
        match cell(rows, position) {
            Some(value) if value == start_value => current = step(position, axis, direction),
            Some(_) => return position,
            None => return start,
        }
    }
    start
}

fn step(position: Position, axis: Axis, direction: Direction) -> Option<Position> {
    Some(match (axis, direction) {
        (Axis::Row, Direction::Forward) => Position {
            row: position.row.checked_add(1)?,
            column: position.column,
        },
        (Axis::Row, Direction::Backward) => Position {
            row: position.row.checked_sub(1)?,
            column: position.column,
        },
        (Axis::Column, Direction::Forward) => Position {
            row: position.row,
            column: position.column.checked_add(1)?,
        },
        (Axis::Column, Direction::Backward) => Position {
            row: position.row,
            column: position.column.checked_sub(1)?,
        },
    })
}

fn cell(rows: &[Vec<String>], position: Position) -> Option<&str> {
    rows.get(position.row)?
        .get(position.column)
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_to_row_value_change() {
        let rows = vec![
            vec!["a".to_owned()],
            vec!["a".to_owned()],
            vec!["b".to_owned()],
        ];
        assert_eq!(
            skip_to_change(
                &rows,
                Position { row: 0, column: 0 },
                Axis::Row,
                Direction::Forward,
                1,
            ),
            Position { row: 2, column: 0 }
        );
    }

    #[test]
    fn skips_to_column_value_change() {
        let rows = vec![vec!["a".to_owned(), "a".to_owned(), "b".to_owned()]];
        assert_eq!(
            skip_to_change(
                &rows,
                Position { row: 0, column: 0 },
                Axis::Column,
                Direction::Forward,
                1,
            ),
            Position { row: 0, column: 2 }
        );
    }
}
