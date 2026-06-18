use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Quit,
    Reload,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Help,
    PageUp,
    PageDown,
    PageLeft,
    PageRight,
    LineHome,
    LineEnd,
    GotoTop,
    GotoRow,
    GotoColumn,
    Mark,
    GotoMark,
    ShowCell,
    Search,
    NextSearchResult,
    PreviousSearchResult,
    ToggleHeader,
    GapDown,
    GapUp,
    AllColumnsNarrower,
    AllColumnsWider,
    CurrentColumnNarrower,
    CurrentColumnWider,
    SortNaturalAsc,
    SortNaturalDesc,
    SortNumericAsc,
    SortNumericDesc,
    SortLexicalAsc,
    SortLexicalDesc,
    YankCell,
    ToggleColumnWidthMode,
    SetCurrentColumnWidth,
    SkipRowChangeForward,
    SkipRowChangeBackward,
    SkipColumnChangeForward,
    SkipColumnChangeBackward,
    ShowInfo,
    Redraw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyAction {
    pub command: Command,
    pub count: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    pub keys: &'static str,
    pub command: Command,
    pub description: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyInterpreter {
    modifier: String,
}

impl KeyInterpreter {
    pub fn handle_char(&mut self, ch: char) -> Option<KeyAction> {
        if ch.is_ascii_digit() && (!self.modifier.is_empty() || lookup_char(ch).is_none()) {
            self.modifier.push(ch);
            return None;
        }

        let command = lookup_char(ch)?;
        let count = if self.modifier.is_empty() {
            None
        } else {
            let count = self.modifier.parse().ok();
            self.modifier.clear();
            count
        };
        Some(KeyAction { command, count })
    }
}

pub fn lookup_char(ch: char) -> Option<Command> {
    Some(match ch {
        'q' | 'Q' => Command::Quit,
        'r' => Command::Reload,
        'j' => Command::MoveDown,
        'k' => Command::MoveUp,
        'h' => Command::MoveLeft,
        'l' => Command::MoveRight,
        'J' => Command::PageDown,
        'K' => Command::PageUp,
        'H' => Command::PageLeft,
        'L' => Command::PageRight,
        '^' => Command::LineHome,
        '$' => Command::LineEnd,
        'g' => Command::GotoTop,
        'G' => Command::GotoRow,
        '|' => Command::GotoColumn,
        'm' => Command::Mark,
        '\'' => Command::GotoMark,
        '\n' => Command::ShowCell,
        '/' => Command::Search,
        'n' => Command::NextSearchResult,
        'p' => Command::PreviousSearchResult,
        't' => Command::ToggleHeader,
        '-' => Command::GapDown,
        '+' => Command::GapUp,
        '<' => Command::AllColumnsNarrower,
        '>' => Command::AllColumnsWider,
        ',' => Command::CurrentColumnNarrower,
        '.' => Command::CurrentColumnWider,
        'a' => Command::SortNaturalAsc,
        'A' => Command::SortNaturalDesc,
        '#' => Command::SortNumericAsc,
        '@' => Command::SortNumericDesc,
        's' => Command::SortLexicalAsc,
        'S' => Command::SortLexicalDesc,
        'y' => Command::YankCell,
        'c' => Command::ToggleColumnWidthMode,
        'C' => Command::SetCurrentColumnWidth,
        ']' => Command::SkipRowChangeForward,
        '[' => Command::SkipRowChangeBackward,
        '}' => Command::SkipColumnChangeForward,
        '{' => Command::SkipColumnChangeBackward,
        '?' => Command::Help,
        _ => return None,
    })
}

pub fn lookup_key_event(event: KeyEvent) -> Option<Command> {
    if event.modifiers == KeyModifiers::CONTROL {
        return match event.code {
            KeyCode::Char('a') => Some(Command::LineHome),
            KeyCode::Char('e') => Some(Command::LineEnd),
            KeyCode::Char('g') => Some(Command::ShowInfo),
            KeyCode::Char('l') => Some(Command::Redraw),
            _ => None,
        };
    }

    match event.code {
        KeyCode::Char(ch) => lookup_char(ch),
        KeyCode::Enter => Some(Command::ShowCell),
        KeyCode::Up => Some(Command::MoveUp),
        KeyCode::Down => Some(Command::MoveDown),
        KeyCode::Left => Some(Command::MoveLeft),
        KeyCode::Right => Some(Command::MoveRight),
        KeyCode::Home => Some(Command::LineHome),
        KeyCode::End => Some(Command::LineEnd),
        KeyCode::PageUp => Some(Command::PageUp),
        KeyCode::PageDown => Some(Command::PageDown),
        KeyCode::Insert => Some(Command::Mark),
        KeyCode::Delete => Some(Command::GotoMark),
        KeyCode::F(1) => Some(Command::Help),
        _ => None,
    }
}

pub fn default_key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding {
            keys: "F1/?",
            command: Command::Help,
            description: "Show keybindings",
        },
        KeyBinding {
            keys: "h/j/k/l",
            command: Command::MoveDown,
            description: "Move selection",
        },
        KeyBinding {
            keys: "Home/^/C-a",
            command: Command::LineHome,
            description: "Move to start of row",
        },
        KeyBinding {
            keys: "End/$/C-e",
            command: Command::LineEnd,
            description: "Move to end of row",
        },
        KeyBinding {
            keys: "[num]|",
            command: Command::GotoColumn,
            description: "Go to column",
        },
        KeyBinding {
            keys: "PgUp/PgDn/J/K",
            command: Command::PageDown,
            description: "Move a page vertically",
        },
        KeyBinding {
            keys: "H/L",
            command: Command::PageLeft,
            description: "Move a page horizontally",
        },
        KeyBinding {
            keys: "g/[num]G",
            command: Command::GotoRow,
            description: "Go to top, row, or bottom",
        },
        KeyBinding {
            keys: "C-g",
            command: Command::ShowInfo,
            description: "Show file/data information",
        },
        KeyBinding {
            keys: "Insert/m",
            command: Command::Mark,
            description: "Mark current cell",
        },
        KeyBinding {
            keys: "Delete/'",
            command: Command::GotoMark,
            description: "Return to mark",
        },
        KeyBinding {
            keys: "Enter",
            command: Command::ShowCell,
            description: "Show full cell contents",
        },
        KeyBinding {
            keys: "/",
            command: Command::Search,
            description: "Search",
        },
        KeyBinding {
            keys: "n/p",
            command: Command::NextSearchResult,
            description: "Next/previous search result",
        },
        KeyBinding {
            keys: "t",
            command: Command::ToggleHeader,
            description: "Toggle header row",
        },
        KeyBinding {
            keys: "</>",
            command: Command::AllColumnsNarrower,
            description: "Resize all columns",
        },
        KeyBinding {
            keys: ",/.",
            command: Command::CurrentColumnNarrower,
            description: "Resize current column",
        },
        KeyBinding {
            keys: "-/+",
            command: Command::GapDown,
            description: "Adjust column gap",
        },
        KeyBinding {
            keys: "s/S",
            command: Command::SortLexicalAsc,
            description: "Lexical sort current column",
        },
        KeyBinding {
            keys: "a/A",
            command: Command::SortNaturalAsc,
            description: "Natural sort current column",
        },
        KeyBinding {
            keys: "#/@",
            command: Command::SortNumericAsc,
            description: "Numeric sort current column",
        },
        KeyBinding {
            keys: "r",
            command: Command::Reload,
            description: "Reload data",
        },
        KeyBinding {
            keys: "y",
            command: Command::YankCell,
            description: "Yank current cell",
        },
        KeyBinding {
            keys: "[num]c/C",
            command: Command::ToggleColumnWidthMode,
            description: "Set column width mode",
        },
        KeyBinding {
            keys: "[num][]",
            command: Command::SkipRowChangeForward,
            description: "Skip row value changes",
        },
        KeyBinding {
            keys: "[num]{}",
            command: Command::SkipColumnChangeForward,
            description: "Skip column value changes",
        },
        KeyBinding {
            keys: "q",
            command: Command::Quit,
            description: "Quit",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_existing_vim_navigation_keys() {
        assert_eq!(lookup_char('j'), Some(Command::MoveDown));
        assert_eq!(lookup_char('k'), Some(Command::MoveUp));
        assert_eq!(lookup_char('h'), Some(Command::MoveLeft));
        assert_eq!(lookup_char('l'), Some(Command::MoveRight));
    }

    #[test]
    fn maps_existing_operation_keys() {
        assert_eq!(lookup_char('r'), Some(Command::Reload));
        assert_eq!(lookup_char('/'), Some(Command::Search));
        assert_eq!(lookup_char('#'), Some(Command::SortNumericAsc));
        assert_eq!(lookup_char('y'), Some(Command::YankCell));
        assert_eq!(lookup_char('?'), Some(Command::Help));
    }

    #[test]
    fn maps_special_keys() {
        assert_eq!(
            lookup_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Some(Command::MoveUp)
        );
        assert_eq!(
            lookup_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            Some(Command::PageDown)
        );
        assert_eq!(
            lookup_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)),
            Some(Command::Help)
        );
    }

    #[test]
    fn maps_control_keys() {
        assert_eq!(
            lookup_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Some(Command::LineHome)
        );
        assert_eq!(
            lookup_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)),
            Some(Command::ShowInfo)
        );
    }

    #[test]
    fn exposes_help_bindings_from_registry() {
        let bindings = default_key_bindings();
        assert!(bindings
            .iter()
            .any(|binding| binding.command == Command::Search));
        assert!(bindings.iter().any(|binding| binding.keys == "q"));
    }

    #[test]
    fn accumulates_numeric_modifier_before_command() {
        let mut interpreter = KeyInterpreter::default();
        assert_eq!(interpreter.handle_char('1'), None);
        assert_eq!(interpreter.handle_char('2'), None);
        assert_eq!(
            interpreter.handle_char('G'),
            Some(KeyAction {
                command: Command::GotoRow,
                count: Some(12)
            })
        );
    }

    #[test]
    fn digit_command_without_modifier_is_ignored_like_unknown_key() {
        let mut interpreter = KeyInterpreter::default();
        assert_eq!(interpreter.handle_char('0'), None);
    }
}
