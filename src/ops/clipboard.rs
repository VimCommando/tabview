use crate::view::Position;

pub fn yank_cell(rows: &[Vec<String>], position: Position) -> anyhow::Result<bool> {
    let Some(text) = rows
        .get(position.row)
        .and_then(|row| row.get(position.column))
    else {
        return Ok(false);
    };
    set_text(text)?;
    Ok(true)
}

#[cfg(feature = "clipboard")]
pub fn set_text(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text.to_owned())?;
    Ok(())
}

#[cfg(not(feature = "clipboard"))]
pub fn set_text(_text: &str) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "clipboard"))]
    #[test]
    fn yanks_existing_cell_without_clipboard_feature() {
        let rows = vec![vec!["cell".to_owned()]];
        assert!(yank_cell(&rows, Position { row: 0, column: 0 }).expect("yank"));
    }

    #[test]
    fn missing_cell_is_non_fatal() {
        let rows = vec![vec!["cell".to_owned()]];
        assert!(!yank_cell(&rows, Position { row: 10, column: 0 }).expect("yank"));
    }
}
