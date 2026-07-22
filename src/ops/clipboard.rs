pub fn yank_text(text: Option<&str>) -> anyhow::Result<bool> {
    let Some(text) = text else {
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
    fn yanks_existing_text_without_clipboard_feature() {
        assert!(yank_text(Some("cell")).expect("yank"));
    }

    #[test]
    fn missing_cell_is_non_fatal() {
        assert!(!yank_text(None).expect("yank"));
    }
}
