use std::cmp::Ordering;
use std::collections::HashSet;

pub(crate) const NUMERIC_PROFILE_SAMPLE_ROWS: usize = 8_192;
const NUMERIC_PROFILE_MAX_SUFFIXES: usize = 32;

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
    let numeric_profile =
        (mode == SortMode::Numeric).then(|| infer_numeric_column_profile(None, rows, column));
    rows.sort_by(|left, right| {
        let ordering = compare_cells(
            left.get(column).map(String::as_str).unwrap_or_default(),
            right.get(column).map(String::as_str).unwrap_or_default(),
            mode,
            numeric_profile.unwrap_or_default(),
        );
        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

pub(crate) fn sort_rows_with_numeric_profile(
    rows: &mut [Vec<String>],
    column: usize,
    direction: SortDirection,
    profile: NumericColumnProfile,
) {
    rows.sort_by(|left, right| {
        let ordering = compare_numeric_cells(
            left.get(column).map(String::as_str).unwrap_or_default(),
            right.get(column).map(String::as_str).unwrap_or_default(),
            profile,
        );
        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

fn compare_cells(
    left: &str,
    right: &str,
    mode: SortMode,
    numeric_profile: NumericColumnProfile,
) -> Ordering {
    match mode {
        SortMode::Lexical => left.cmp(right),
        SortMode::Natural => natural_tokens(left).cmp(&natural_tokens(right)),
        SortMode::Numeric => compare_numeric_cells(left, right, numeric_profile),
    }
}

pub(crate) fn compare_numeric_cells(
    left: &str,
    right: &str,
    profile: NumericColumnProfile,
) -> Ordering {
    match (
        parse_numeric_key(left, profile),
        parse_numeric_key(right, profile),
    ) {
        (Some(NumericKey::MultiPart(left)), Some(NumericKey::MultiPart(right))) => {
            compare_numeric_parts(&left, &right)
        }
        (Some(NumericKey::Scalar(left)), Some(NumericKey::Scalar(right))) => left.total_cmp(&right),
        (Some(NumericKey::MultiPart(left)), Some(NumericKey::Scalar(right))) => {
            compare_parts_to_scalar(&left, right)
        }
        (Some(NumericKey::Scalar(left)), Some(NumericKey::MultiPart(right))) => {
            compare_parts_to_scalar(&right, left).reverse()
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NumericKey {
    Scalar(f64),
    MultiPart(Vec<u64>),
}

fn parse_numeric_key(value: &str, profile: NumericColumnProfile) -> Option<NumericKey> {
    if is_numeric_placeholder(value) {
        return None;
    }

    parse_multi_dot_number(value)
        .map(NumericKey::MultiPart)
        .or_else(|| parse_suffixed_number_with_profile(value, profile).map(NumericKey::Scalar))
}

pub(crate) fn is_numeric_cell(value: &str, profile: NumericColumnProfile) -> bool {
    parse_numeric_key(value, profile).is_some()
}

pub(crate) fn parse_numeric_scalar(value: &str, profile: NumericColumnProfile) -> Option<f64> {
    if is_numeric_placeholder(value) {
        return None;
    }
    parse_suffixed_number_with_profile(value, profile)
}

pub(crate) fn is_numeric_placeholder(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "null" | "n/a" | "na" | "none" | "nil" | "nan"
    )
}

fn parse_multi_dot_number(value: &str) -> Option<Vec<u64>> {
    let value = value.trim();
    if value.matches('.').count() < 2 {
        return None;
    }

    value
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }
            part.parse::<u64>().ok()
        })
        .collect()
}

fn compare_numeric_parts(left: &[u64], right: &[u64]) -> Ordering {
    for idx in 0..left.len().max(right.len()) {
        let left = left.get(idx).copied().unwrap_or(0);
        let right = right.get(idx).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

fn compare_parts_to_scalar(parts: &[u64], scalar: f64) -> Ordering {
    if scalar.is_finite() && scalar >= 0.0 && scalar.fract() == 0.0 && scalar <= u64::MAX as f64 {
        return compare_numeric_parts(parts, &[scalar as u64]);
    }

    let first_part = parts.first().copied().unwrap_or(0) as f64;
    first_part.total_cmp(&scalar)
}

#[cfg(test)]
fn parse_suffixed_number(value: &str) -> Option<f64> {
    parse_suffixed_number_with_profile(value, NumericColumnProfile::default())
}

pub(crate) fn parse_suffixed_number_with_profile(
    value: &str,
    profile: NumericColumnProfile,
) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(number) = value.parse::<f64>() {
        return Some(number);
    }

    value
        .char_indices()
        .map(|(idx, _)| idx)
        .chain(std::iter::once(value.len()))
        .filter(|idx| *idx > 0)
        .rev()
        .find_map(|idx| {
            let (number, suffix) = value.split_at(idx);
            let number = number.trim_end().parse::<f64>().ok()?;
            let multiplier = suffix_multiplier(suffix.trim_start(), profile)?;
            Some(number * multiplier)
        })
}

fn suffix_multiplier(suffix: &str, profile: NumericColumnProfile) -> Option<f64> {
    percent_suffix_multiplier(suffix)
        .or_else(|| byte_suffix_multiplier(suffix))
        .or_else(|| time_suffix_multiplier(suffix, profile))
        .or_else(|| scientific_suffix_multiplier(suffix, profile))
}

fn percent_suffix_multiplier(suffix: &str) -> Option<f64> {
    (suffix == "%").then_some(1.0)
}

fn byte_suffix_multiplier(suffix: &str) -> Option<f64> {
    let suffix = suffix.to_ascii_lowercase();
    match suffix.as_str() {
        "b" | "byte" | "bytes" => Some(1.0),
        "kb" | "kilobyte" | "kilobytes" => Some(1_000.0),
        "mb" | "megabyte" | "megabytes" => Some(1_000_000.0),
        "gb" | "gigabyte" | "gigabytes" => Some(1_000_000_000.0),
        "tb" | "terabyte" | "terabytes" => Some(1_000_000_000_000.0),
        "pb" | "petabyte" | "petabytes" => Some(1_000_000_000_000_000.0),
        "eb" | "exabyte" | "exabytes" => Some(1_000_000_000_000_000_000.0),
        "kib" | "kibibyte" | "kibibytes" => Some(1024.0),
        "mib" | "mebibyte" | "mebibytes" => Some(1024.0_f64.powi(2)),
        "gib" | "gibibyte" | "gibibytes" => Some(1024.0_f64.powi(3)),
        "tib" | "tebibyte" | "tebibytes" => Some(1024.0_f64.powi(4)),
        "pib" | "pebibyte" | "pebibytes" => Some(1024.0_f64.powi(5)),
        "eib" | "exbibyte" | "exbibytes" => Some(1024.0_f64.powi(6)),
        _ => None,
    }
}

fn scientific_suffix_multiplier(suffix: &str, profile: NumericColumnProfile) -> Option<f64> {
    match suffix {
        "n" => Some(0.000_000_001),
        "u" | "mu" | "µ" | "μ" => Some(0.000_001),
        "m" if !profile.bare_m_is_minutes() => Some(0.001),
        "k" => Some(1_000.0),
        "M" => Some(1_000_000.0),
        "g" | "G" => Some(1_000_000_000.0),
        "T" => Some(1_000_000_000_000.0),
        "P" => Some(1_000_000_000_000_000.0),
        "E" => Some(1_000_000_000_000_000_000.0),
        _ => None,
    }
}

fn time_suffix_multiplier(suffix: &str, profile: NumericColumnProfile) -> Option<f64> {
    let suffix = suffix.to_ascii_lowercase();
    match suffix.as_str() {
        "ns" | "nanosecond" | "nanoseconds" => Some(0.000_000_001),
        "us" | "µs" | "μs" | "mus" | "microsecond" | "microseconds" => Some(0.000_001),
        "ms" | "millisecond" | "milliseconds" => Some(0.001),
        "s" | "sec" | "secs" | "second" | "seconds" => Some(1.0),
        "m" if profile.bare_m_is_minutes() => Some(60.0),
        "min" | "mins" | "minute" | "minutes" => Some(60.0),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(60.0 * 60.0),
        "d" | "day" | "days" => Some(60.0 * 60.0 * 24.0),
        "y" | "yr" | "yrs" | "year" | "years" => Some(60.0 * 60.0 * 24.0 * 365.25),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NumericColumnProfile {
    kind: NumericColumnKind,
}

impl NumericColumnProfile {
    pub(crate) fn time() -> Self {
        Self {
            kind: NumericColumnKind::Time,
        }
    }

    fn bare_m_is_minutes(self) -> bool {
        self.kind == NumericColumnKind::Time
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum NumericColumnKind {
    #[default]
    Default,
    Time,
}

pub(crate) fn infer_numeric_column_profile(
    header: Option<&str>,
    rows: &[Vec<String>],
    column: usize,
) -> NumericColumnProfile {
    if header.is_some_and(header_suggests_time) {
        return NumericColumnProfile::time();
    }

    let mut evidence = SuffixEvidence::default();

    for row in rows.iter().take(NUMERIC_PROFILE_SAMPLE_ROWS) {
        let Some(cell) = row.get(column).map(|cell| cell.trim()) else {
            continue;
        };
        if cell.is_empty() {
            continue;
        }
        if let Some(suffix) = numeric_suffix(cell) {
            evidence.record(suffix);
        }
        if evidence.is_confident() {
            break;
        }
    }

    if evidence.has_time {
        NumericColumnProfile::time()
    } else {
        NumericColumnProfile::default()
    }
}

#[derive(Debug, Default)]
struct SuffixEvidence {
    suffixes: HashSet<String>,
    has_time: bool,
}

impl SuffixEvidence {
    fn record(&mut self, suffix: &str) {
        if suffix.is_empty() {
            return;
        }
        self.suffixes.insert(normalize_suffix(suffix));
        if is_time_context_suffix(suffix) {
            self.has_time = true;
        }
    }

    fn is_confident(&self) -> bool {
        self.has_time || self.suffixes.len() >= NUMERIC_PROFILE_MAX_SUFFIXES
    }
}

fn numeric_suffix(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() || value.parse::<f64>().is_ok() {
        return None;
    }

    value
        .char_indices()
        .map(|(idx, _)| idx)
        .chain(std::iter::once(value.len()))
        .filter(|idx| *idx > 0)
        .rev()
        .find_map(|idx| {
            let (number, suffix) = value.split_at(idx);
            number
                .trim_end()
                .parse::<f64>()
                .ok()
                .map(|_| suffix.trim_start())
        })
}

fn normalize_suffix(suffix: &str) -> String {
    suffix.trim().to_ascii_lowercase()
}

fn is_time_context_suffix(suffix: &str) -> bool {
    let suffix = suffix.to_ascii_lowercase();
    matches!(
        suffix.as_str(),
        "ns" | "nanosecond"
            | "nanoseconds"
            | "us"
            | "µs"
            | "μs"
            | "mus"
            | "microsecond"
            | "microseconds"
            | "ms"
            | "millisecond"
            | "milliseconds"
            | "s"
            | "sec"
            | "secs"
            | "second"
            | "seconds"
            | "min"
            | "mins"
            | "minute"
            | "minutes"
            | "h"
            | "hr"
            | "hrs"
            | "hour"
            | "hours"
            | "d"
            | "day"
            | "days"
            | "y"
            | "yr"
            | "yrs"
            | "year"
            | "years"
    )
}

fn header_suggests_time(header: &str) -> bool {
    let header = header.to_ascii_lowercase();
    let tokens = header
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != 'µ' && ch != 'μ')
        .filter(|token| !token.is_empty());

    tokens.clone().any(is_time_header_token)
        || [
            "duration", "latency", "elapsed", "runtime", "uptime", "timeout", "interval",
        ]
        .iter()
        .any(|hint| header.contains(hint))
}

fn is_time_header_token(token: &str) -> bool {
    matches!(
        token,
        "ns" | "nanosecond"
            | "nanoseconds"
            | "us"
            | "µs"
            | "μs"
            | "microsecond"
            | "microseconds"
            | "ms"
            | "millisecond"
            | "milliseconds"
            | "s"
            | "sec"
            | "secs"
            | "second"
            | "seconds"
            | "min"
            | "mins"
            | "minute"
            | "minutes"
            | "h"
            | "hr"
            | "hrs"
            | "hour"
            | "hours"
            | "d"
            | "day"
            | "days"
            | "y"
            | "yr"
            | "yrs"
            | "year"
            | "years"
    )
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

    #[test]
    fn sorts_ip_like_values_by_numeric_components() {
        let mut rows = vec![
            vec!["10.0.0.10".to_owned()],
            vec!["10.0.0.2".to_owned()],
            vec!["192.168.0.1".to_owned()],
            vec!["10.0.0.1".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["10.0.0.1".to_owned()],
                vec!["10.0.0.2".to_owned()],
                vec!["10.0.0.10".to_owned()],
                vec!["192.168.0.1".to_owned()],
            ]
        );
    }

    #[test]
    fn sorts_semver_like_values_by_numeric_components() {
        let mut rows = vec![
            vec!["1.10.0".to_owned()],
            vec!["1.2.10".to_owned()],
            vec!["1.2.3".to_owned()],
            vec!["2.0.0".to_owned()],
            vec!["1.2.0".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1.2.0".to_owned()],
                vec!["1.2.3".to_owned()],
                vec!["1.2.10".to_owned()],
                vec!["1.10.0".to_owned()],
                vec!["2.0.0".to_owned()],
            ]
        );
    }

    #[test]
    fn keeps_single_dot_values_as_decimals() {
        let mut rows = vec![
            vec!["1.10".to_owned()],
            vec!["1.2".to_owned()],
            vec!["1.02".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1.02".to_owned()],
                vec!["1.10".to_owned()],
                vec!["1.2".to_owned()],
            ]
        );
    }

    #[test]
    fn sorts_numerically_with_scientific_suffixes() {
        let mut rows = vec![
            vec!["2M".to_owned()],
            vec!["3k".to_owned()],
            vec!["2u".to_owned()],
            vec!["2m".to_owned()],
            vec!["1g".to_owned()],
            vec!["1".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["2u".to_owned()],
                vec!["2m".to_owned()],
                vec!["1".to_owned()],
                vec!["3k".to_owned()],
                vec!["2M".to_owned()],
                vec!["1g".to_owned()],
            ]
        );
    }

    #[test]
    fn sorts_numerically_with_byte_suffixes() {
        let mut rows = vec![
            vec!["1GiB".to_owned()],
            vec!["1GB".to_owned()],
            vec!["2mb".to_owned()],
            vec!["512KiB".to_owned()],
            vec!["512kb".to_owned()],
            vec!["1MiB".to_owned()],
            vec!["1mb".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["512kb".to_owned()],
                vec!["512KiB".to_owned()],
                vec!["1mb".to_owned()],
                vec!["1MiB".to_owned()],
                vec!["2mb".to_owned()],
                vec!["1GB".to_owned()],
                vec!["1GiB".to_owned()],
            ]
        );
    }

    #[test]
    fn byte_suffix_takes_precedence_over_single_letter_suffix() {
        assert_eq!(parse_suffixed_number("1mb"), Some(1_000_000.0));
        assert_eq!(parse_suffixed_number("1m"), Some(0.001));
    }

    #[test]
    fn sorts_numerically_with_expanded_scientific_suffixes() {
        let mut rows = vec![
            vec!["1E".to_owned()],
            vec!["1T".to_owned()],
            vec!["1mu".to_owned()],
            vec!["1P".to_owned()],
            vec!["1n".to_owned()],
            vec!["1µ".to_owned()],
            vec!["1M".to_owned()],
            vec!["1m".to_owned()],
            vec!["1k".to_owned()],
            vec!["1G".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1n".to_owned()],
                vec!["1mu".to_owned()],
                vec!["1µ".to_owned()],
                vec!["1m".to_owned()],
                vec!["1k".to_owned()],
                vec!["1M".to_owned()],
                vec!["1G".to_owned()],
                vec!["1T".to_owned()],
                vec!["1P".to_owned()],
                vec!["1E".to_owned()],
            ]
        );
    }

    #[test]
    fn sorts_numerically_with_time_suffixes() {
        let mut rows = vec![
            vec!["1year".to_owned()],
            vec!["1day".to_owned()],
            vec!["1h".to_owned()],
            vec!["1min".to_owned()],
            vec!["1s".to_owned()],
            vec!["1ms".to_owned()],
            vec!["1us".to_owned()],
            vec!["1ns".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1ns".to_owned()],
                vec!["1us".to_owned()],
                vec!["1ms".to_owned()],
                vec!["1s".to_owned()],
                vec!["1min".to_owned()],
                vec!["1h".to_owned()],
                vec!["1day".to_owned()],
                vec!["1year".to_owned()],
            ]
        );
    }

    #[test]
    fn sorts_numerically_with_percent_suffixes() {
        let mut rows = vec![
            vec!["100%".to_owned()],
            vec!["2%".to_owned()],
            vec!["2.5%".to_owned()],
            vec!["10%".to_owned()],
            vec!["1".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1".to_owned()],
                vec!["2%".to_owned()],
                vec!["2.5%".to_owned()],
                vec!["10%".to_owned()],
                vec!["100%".to_owned()],
            ]
        );
    }

    #[test]
    fn sorts_numeric_placeholders_after_numbers() {
        let mut rows = vec![
            vec!["n/a".to_owned()],
            vec!["2.5%".to_owned()],
            vec!["null".to_owned()],
            vec!["1".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1".to_owned()],
                vec!["2.5%".to_owned()],
                vec!["n/a".to_owned()],
                vec!["null".to_owned()],
            ]
        );
    }

    #[test]
    fn treats_bare_m_as_minutes_in_time_context() {
        let mut rows = vec![
            vec!["1h".to_owned()],
            vec!["2m".to_owned()],
            vec!["30s".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["30s".to_owned()],
                vec!["2m".to_owned()],
                vec!["1h".to_owned()],
            ]
        );
    }

    #[test]
    fn header_hint_treats_bare_m_as_minutes() {
        let mut rows = vec![
            vec!["2m".to_owned()],
            vec!["30".to_owned()],
            vec!["1".to_owned()],
        ];
        let profile = infer_numeric_column_profile(Some("duration"), &rows, 0);
        sort_rows_with_numeric_profile(&mut rows, 0, SortDirection::Ascending, profile);
        assert_eq!(
            rows,
            vec![
                vec!["1".to_owned()],
                vec!["30".to_owned()],
                vec!["2m".to_owned()],
            ]
        );
    }

    #[test]
    fn treats_bare_m_as_milli_in_scientific_context() {
        let mut rows = vec![
            vec!["3k".to_owned()],
            vec!["1".to_owned()],
            vec!["2m".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["2m".to_owned()],
                vec!["1".to_owned()],
                vec!["3k".to_owned()],
            ]
        );
    }

    #[test]
    fn leaves_bare_m_as_milli_when_ambiguous() {
        let mut rows = vec![
            vec!["2m".to_owned()],
            vec!["1".to_owned()],
            vec!["1m".to_owned()],
        ];
        sort_rows(&mut rows, 0, SortMode::Numeric, SortDirection::Ascending);
        assert_eq!(
            rows,
            vec![
                vec!["1m".to_owned()],
                vec!["2m".to_owned()],
                vec!["1".to_owned()],
            ]
        );
    }

    #[test]
    fn byte_suffixes_are_case_insensitive() {
        assert_eq!(parse_suffixed_number("1kb"), parse_suffixed_number("1KB"));
        assert_eq!(parse_suffixed_number("1mib"), parse_suffixed_number("1MiB"));
        assert_eq!(
            parse_suffixed_number("1mebibyte"),
            parse_suffixed_number("1MEBIBYTE")
        );
    }
}
