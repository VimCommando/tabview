use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use tabview::compat::{Comparison, CompatibilityClass};
use tabview::ingest::{parse_rows, ParseOptions};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn python_oracle(args: &[&str]) -> Value {
    let output = Command::new("python3")
        .arg(repo_root().join("tests/compat/python_oracle.py"))
        .args(args)
        .output()
        .expect("run python compatibility oracle");

    assert!(
        output.status.success(),
        "oracle failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("oracle JSON")
}

fn oracle_rows(args: &[&str]) -> Vec<Vec<String>> {
    let output = python_oracle(args);
    serde_json::from_value(output["rows"].clone()).expect("oracle rows")
}

fn read_fixture(path: &str) -> Vec<u8> {
    std::fs::read(repo_root().join(path)).expect("fixture bytes")
}

#[test]
fn python_oracle_detects_utf8_sample() {
    let output = python_oracle(&["detect-encoding", "sample/unicode-example-utf8.txt"]);
    assert_eq!(output["encoding"], "utf-8");
}

#[test]
fn python_oracle_processes_space_delimited_sample() {
    let output = python_oracle(&[
        "process-data",
        "sample/commented_annotated_numeric.txt",
        "--encoding",
        "utf-8",
    ]);
    assert_eq!(output["rows"][0][0], "A");
    let rows = output["rows"].as_array().expect("rows array");
    assert_eq!(rows.last().expect("last row")[2], "+3");
}

#[test]
fn fixture_matrix_lists_required_accepted_changes() {
    let fixture_path = repo_root().join("tests/compat/fixtures/cases.json");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(fixture_path).expect("fixture file"))
            .expect("fixture JSON");
    let accepted = fixture["accepted_changes"]
        .as_array()
        .expect("accepted_changes array");

    for name in [
        "macos_clipboard_without_display",
        "empty_cell_popup_noop",
        "multi_row_csv_sniffing",
        "structural_header_toggle",
        "non_mutating_reverse_search",
        "specific_encoding_before_latin1",
        "default_mode_column_width",
    ] {
        assert!(
            accepted.iter().any(|item| item == name),
            "missing accepted-change fixture {name}"
        );
    }
}

#[test]
fn compatibility_comparison_classifies_regressions() {
    let comparison = Comparison::compatible("sample", "python", "rust");
    assert_eq!(comparison.class, CompatibilityClass::Regression);
    assert!(comparison.class.is_failure());
}

#[test]
fn compatibility_comparison_allows_intentional_enhancements() {
    let comparison = Comparison::accepted_change(
        "default width",
        "20",
        "mode",
        CompatibilityClass::IntentionalEnhancement,
    );
    assert_eq!(comparison.class, CompatibilityClass::IntentionalEnhancement);
    assert!(!comparison.class.is_failure());
}

#[test]
fn rust_parser_matches_python_utf8_sample() {
    let path = "sample/unicode-example-utf8.txt";
    let expected = oracle_rows(&["process-data", path, "--encoding", "utf-8"]);
    let actual = parse_rows(
        &read_fixture(path),
        &ParseOptions {
            encoding: Some("utf-8".to_owned()),
            ..ParseOptions::default()
        },
    )
    .expect("rust rows");
    assert_eq!(
        Comparison::compatible(path, expected, actual).class,
        CompatibilityClass::Compatible
    );
}

#[test]
fn rust_parser_matches_python_latin1_sample_with_explicit_encoding() {
    let path = "sample/test_latin-1.csv";
    let expected = oracle_rows(&["process-data", path, "--encoding", "latin-1"]);
    let actual = parse_rows(
        &read_fixture(path),
        &ParseOptions {
            encoding: Some("latin-1".to_owned()),
            ..ParseOptions::default()
        },
    )
    .expect("rust rows");
    assert_eq!(
        Comparison::compatible(path, expected, actual).class,
        CompatibilityClass::Compatible
    );
}
