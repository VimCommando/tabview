use std::fmt;
use std::str::FromStr;

use super::ParseOptions;

pub const DEFAULT_SCHEMA_SCAN_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputFormat {
    #[default]
    Auto,
    Delimited,
    Json,
    Ndjson,
}

impl FromStr for InputFormat {
    type Err = SourceOptionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "delimited" => Ok(Self::Delimited),
            "json" => Ok(Self::Json),
            "ndjson" => Ok(Self::Ndjson),
            _ => Err(SourceOptionError::InvalidFormat(value.to_owned())),
        }
    }
}

impl fmt::Display for InputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Auto => "auto",
            Self::Delimited => "delimited",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SchemaScan {
    #[default]
    Default,
    Full,
}

impl FromStr for SchemaScan {
    type Err = SourceOptionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "default" => Ok(Self::Default),
            "full" => Ok(Self::Full),
            _ => Err(SourceOptionError::InvalidSchemaScan(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPointer {
    encoded: String,
    segments: Vec<String>,
}

impl JsonPointer {
    pub fn root() -> Self {
        Self {
            encoded: String::new(),
            segments: Vec::new(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.encoded
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn encode_segment(segment: &str) -> String {
        segment.replace('~', "~0").replace('/', "~1")
    }
}

impl FromStr for JsonPointer {
    type Err = SourceOptionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Ok(Self::root());
        }
        if !value.starts_with('/') {
            return Err(SourceOptionError::InvalidJsonPointer(value.to_owned()));
        }
        let segments = value[1..]
            .split('/')
            .map(|segment| decode_pointer_segment(segment, value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            encoded: value.to_owned(),
            segments,
        })
    }
}

fn decode_pointer_segment(segment: &str, full: &str) -> Result<String, SourceOptionError> {
    let mut decoded = String::new();
    let mut chars = segment.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            _ => return Err(SourceOptionError::InvalidJsonPointer(full.to_owned())),
        }
    }
    Ok(decoded)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenOptions {
    pub format: InputFormat,
    pub delimited: ParseOptions,
    pub json_path: Option<JsonPointer>,
    pub schema_scan: SchemaScan,
    pub lazy_threshold_bytes: u64,
    pub schema_scan_bytes: u64,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            format: InputFormat::Auto,
            delimited: ParseOptions::default(),
            json_path: None,
            schema_scan: SchemaScan::Default,
            lazy_threshold_bytes: super::DEFAULT_LAZY_THRESHOLD_BYTES,
            schema_scan_bytes: DEFAULT_SCHEMA_SCAN_BYTES,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceOptionOverrides {
    pub format: Option<InputFormat>,
    pub json_path: Option<JsonPointer>,
    pub schema_scan: Option<SchemaScan>,
}

impl OpenOptions {
    pub fn merge(
        defaults: Self,
        saved: &SourceOptionOverrides,
        cli: &SourceOptionOverrides,
    ) -> Self {
        Self {
            format: cli.format.or(saved.format).unwrap_or(defaults.format),
            json_path: cli
                .json_path
                .clone()
                .or_else(|| saved.json_path.clone())
                .or(defaults.json_path),
            schema_scan: cli
                .schema_scan
                .or(saved.schema_scan)
                .unwrap_or(defaults.schema_scan),
            ..defaults
        }
    }

    pub fn validate(&self) -> Result<(), SourceOptionError> {
        if self.format == InputFormat::Delimited && self.json_path.is_some() {
            return Err(SourceOptionError::JsonPathRequiresStructuredFormat);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SourceOptionError {
    #[error("invalid input format '{0}' (expected auto, delimited, json, or ndjson)")]
    InvalidFormat(String),
    #[error("invalid schema scan policy '{0}' (expected default or full)")]
    InvalidSchemaScan(String),
    #[error("invalid RFC 6901 JSON Pointer '{0}'")]
    InvalidJsonPointer(String),
    #[error("JSON starting paths require JSON or NDJSON input")]
    JsonPathRequiresStructuredFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_decodes_json_pointers() {
        let pointer: JsonPointer = "/hits/hits/a~1b/~0meta".parse().expect("pointer");
        assert_eq!(pointer.segments(), ["hits", "hits", "a/b", "~meta"]);
        assert!("hits/hits".parse::<JsonPointer>().is_err());
        assert!("/bad~2escape".parse::<JsonPointer>().is_err());
    }

    #[test]
    fn merges_source_options_in_cli_saved_default_order() {
        let defaults = OpenOptions::default();
        let saved = SourceOptionOverrides {
            format: Some(InputFormat::Json),
            schema_scan: Some(SchemaScan::Full),
            ..SourceOptionOverrides::default()
        };
        let cli = SourceOptionOverrides {
            schema_scan: Some(SchemaScan::Default),
            ..SourceOptionOverrides::default()
        };
        let merged = OpenOptions::merge(defaults, &saved, &cli);
        assert_eq!(merged.format, InputFormat::Json);
        assert_eq!(merged.schema_scan, SchemaScan::Default);
        assert_ne!(merged.lazy_threshold_bytes, 0);
        assert_ne!(merged.schema_scan_bytes, 0);
    }
}
