use std::path::PathBuf;

use clap::{ArgAction, Parser};

use crate::ingest::Quoting;
use crate::view::ColumnWidthMode;

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "tabview",
    about = "View a delimited file in a spreadsheet-like display.",
    disable_help_subcommand = true
)]
pub struct Args {
    /// File to read. Use '-' to read from standard input.
    pub filename: PathBuf,

    /// Encoding, if required.
    #[arg(short = 'e', long = "encoding")]
    pub encoding: Option<String>,

    /// CSV delimiter. Not typically necessary since automatic delimiter sniffing is used.
    #[arg(short = 'd', long = "delimiter")]
    pub delimiter: Option<String>,

    /// CSV quoting style, using Python csv.QUOTE_* names.
    #[arg(long = "quoting")]
    pub quoting: Option<String>,

    /// Initial cursor display position as y or y,x.
    #[arg(short = 's', long = "start_pos")]
    pub start_pos: Option<String>,

    /// Column width: 'max', 'mode', or an integer fixed width.
    #[arg(short = 'w', long = "width", default_value = "mode")]
    pub width: String,

    /// Force full handling of double-width characters for large files.
    #[arg(long = "double_width", action = ArgAction::SetTrue)]
    pub double_width: bool,

    /// Quote character.
    #[arg(short = 'q', long = "quote-char", default_value = "\"")]
    pub quote_char: String,

    /// Force a saved view by canonical name.
    #[cfg(feature = "saved-views")]
    #[arg(long = "view", conflicts_with = "no_view")]
    pub view: Option<String>,

    /// Disable saved view discovery and application for this invocation.
    #[cfg(feature = "saved-views")]
    #[arg(long = "no-view", action = ArgAction::SetTrue)]
    pub no_view: bool,

    /// Extra positional arguments, including classic +y:x start positions.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub extra: Vec<String>,
}

impl Args {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub filename: PathBuf,
    pub encoding: Option<String>,
    pub delimiter: Option<u8>,
    pub quoting: Option<Quoting>,
    pub start_position: StartPosition,
    pub width: ColumnWidthMode,
    pub double_width: bool,
    pub quote_char: char,
    #[cfg(feature = "saved-views")]
    pub saved_view: SavedViewSelection,
}

impl Config {
    pub fn from_args(args: Args) -> Result<Self, CliError> {
        Ok(Self {
            filename: args.filename,
            encoding: args.encoding,
            delimiter: args.delimiter.as_deref().map(parse_byte_char).transpose()?,
            quoting: args.quoting.as_deref().map(parse_quoting).transpose()?,
            start_position: parse_start_position(args.start_pos.as_deref(), &args.extra)?,
            width: parse_width(&args.width)?,
            double_width: args.double_width,
            quote_char: parse_char(&args.quote_char, "quote character")?,
            #[cfg(feature = "saved-views")]
            saved_view: SavedViewSelection::from_args(args.view, args.no_view),
        })
    }
}

#[cfg(feature = "saved-views")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedViewSelection {
    Auto,
    Force(String),
    Disabled,
}

#[cfg(feature = "saved-views")]
impl SavedViewSelection {
    fn from_args(view: Option<String>, no_view: bool) -> Self {
        if no_view {
            Self::Disabled
        } else if let Some(view) = view {
            Self::Force(crate::saved_views::normalize_view_name(&view))
        } else {
            Self::Auto
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StartPosition {
    pub row: usize,
    pub column: Option<usize>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CliError {
    #[error("invalid start position '{value}'")]
    InvalidStartPosition { value: String },
    #[error("invalid column width '{value}'")]
    InvalidWidth { value: String },
    #[error("invalid quoting style '{value}'")]
    InvalidQuoting { value: String },
    #[error("invalid {what} '{value}'")]
    InvalidChar { what: &'static str, value: String },
}

fn parse_start_position(normal: Option<&str>, extra: &[String]) -> Result<StartPosition, CliError> {
    if let Some(value) = normal {
        return parse_normal_start_position(value);
    }

    if let Some(value) = extra.iter().find(|value| value.starts_with('+')) {
        return parse_classic_start_position(value);
    }

    Ok(StartPosition::default())
}

fn parse_normal_start_position(value: &str) -> Result<StartPosition, CliError> {
    let mut parts = value.split(',');
    let row = parse_optional_usize(parts.next().unwrap_or_default(), value)?;
    let column = parts
        .next()
        .map(|part| parse_optional_usize(part, value))
        .transpose()?;
    Ok(StartPosition { row, column })
}

fn parse_classic_start_position(value: &str) -> Result<StartPosition, CliError> {
    let value_without_plus = value.trim_start_matches('+');
    let mut parts = value_without_plus.split(':');
    let row = parse_optional_usize(parts.next().unwrap_or_default(), value)?;
    let column = parts
        .next()
        .map(|part| parse_optional_usize(part, value))
        .transpose()?
        .or(Some(0));
    Ok(StartPosition { row, column })
}

fn parse_optional_usize(part: &str, full_value: &str) -> Result<usize, CliError> {
    if part.is_empty() {
        return Ok(0);
    }
    part.parse().map_err(|_| CliError::InvalidStartPosition {
        value: full_value.to_owned(),
    })
}

fn parse_width(value: &str) -> Result<ColumnWidthMode, CliError> {
    match value {
        "mode" => Ok(ColumnWidthMode::Mode),
        "max" => Ok(ColumnWidthMode::Max),
        _ => value
            .parse::<u16>()
            .map(ColumnWidthMode::Fixed)
            .map_err(|_| CliError::InvalidWidth {
                value: value.to_owned(),
            }),
    }
}

fn parse_quoting(value: &str) -> Result<Quoting, CliError> {
    match value {
        "QUOTE_MINIMAL" => Ok(Quoting::Minimal),
        "QUOTE_NONNUMERIC" => Ok(Quoting::NonNumeric),
        "QUOTE_ALL" => Ok(Quoting::All),
        "QUOTE_NONE" => Ok(Quoting::None),
        _ => Err(CliError::InvalidQuoting {
            value: value.to_owned(),
        }),
    }
}

fn parse_byte_char(value: &str) -> Result<u8, CliError> {
    if value == r"\t" {
        return Ok(b'\t');
    }
    let ch = parse_char(value, "delimiter")?;
    if ch.is_ascii() {
        Ok(ch as u8)
    } else {
        Err(CliError::InvalidChar {
            what: "delimiter",
            value: value.to_owned(),
        })
    }
}

fn parse_char(value: &str, what: &'static str) -> Result<char, CliError> {
    let mut chars = value.chars();
    let Some(ch) = chars.next() else {
        return Err(CliError::InvalidChar {
            what,
            value: value.to_owned(),
        });
    };
    if chars.next().is_none() {
        Ok(ch)
    } else {
        Err(CliError::InvalidChar {
            what,
            value: value.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Config {
        let args = Args::try_parse_from(args).expect("parse args");
        Config::from_args(args).expect("config")
    }

    #[test]
    fn default_width_is_mode() {
        let config = parse(&["tabview", "sample/data_ohlcv.csv"]);
        assert_eq!(config.width, ColumnWidthMode::Mode);
    }

    #[test]
    fn parses_readme_start_position() {
        let config = parse(&[
            "tabview",
            "sample/data_ohlcv.csv",
            "--start_pos",
            "6,5",
            "--encoding",
            "utf-8",
        ]);
        assert_eq!(
            config.start_position,
            StartPosition {
                row: 6,
                column: Some(5)
            }
        );
        assert_eq!(config.encoding.as_deref(), Some("utf-8"));
    }

    #[test]
    fn parses_classic_start_position() {
        let config = parse(&["tabview", "sample/data_ohlcv.csv", "+6:5"]);
        assert_eq!(
            config.start_position,
            StartPosition {
                row: 6,
                column: Some(5)
            }
        );
    }

    #[test]
    fn parses_classic_row_only_start_position() {
        let config = parse(&["tabview", "sample/data_ohlcv.csv", "+6:"]);
        assert_eq!(
            config.start_position,
            StartPosition {
                row: 6,
                column: Some(0)
            }
        );
    }

    #[test]
    fn parses_mysql_pager_shape() {
        let config = parse(&["tabview", "-d", r"\t", "--quoting", "QUOTE_NONE", "-"]);
        assert_eq!(config.filename, PathBuf::from("-"));
        assert_eq!(config.delimiter, Some(b'\t'));
        assert_eq!(config.quoting, Some(Quoting::None));
    }

    #[cfg(feature = "saved-views")]
    #[test]
    fn parses_saved_view_selection_flags() {
        let config = parse(&["tabview", "--view", "cat-shards.yml", "sample/data.csv"]);
        assert_eq!(
            config.saved_view,
            SavedViewSelection::Force("cat-shards".to_owned())
        );

        let config = parse(&["tabview", "--no-view", "sample/data.csv"]);
        assert_eq!(config.saved_view, SavedViewSelection::Disabled);

        assert!(Args::try_parse_from([
            "tabview",
            "--view",
            "cat-shards",
            "--no-view",
            "sample/data.csv"
        ])
        .is_err());
    }
}
