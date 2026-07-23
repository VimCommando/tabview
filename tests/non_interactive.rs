use assert_cmd::Command;
use predicates::prelude::*;
use std::io::BufRead;
use std::process::Stdio;

fn fixture(contents: &str, suffix: &str) -> tempfile::NamedTempFile {
    let file = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("temp file");
    std::fs::write(file.path(), contents).expect("write fixture");
    file
}

#[test]
fn direct_table_and_automatic_redirection_match() {
    let file = fixture("Name,Count\nalpha,2\nbeta,10\n", ".csv");
    let expected = "Name   Count\nalpha      2\nbeta      10\n";

    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(expected)
        .stderr("");

    Command::cargo_bin("tabview")
        .expect("binary")
        .arg(file.path())
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn stdin_pipeline_uses_data_stream_without_terminal_access() {
    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table", "-"])
        .write_stdin("A,B\n1,2\n3,4\n")
        .assert()
        .success()
        .stdout("A  B\n1  2\n3  4\n")
        .stderr("");
}

#[test]
fn stdin_pipeline_preserves_keyed_object_modes() {
    let input = r#"{"alpha":{"stars":1},"beta":{"stars":2},"gamma":{"stars":3}}"#;

    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["--format", "json", "-o", "table", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("name   stars\nalpha      1\nbeta       2\ngamma      3\n");

    Command::cargo_bin("tabview")
        .expect("binary")
        .args([
            "--format",
            "json",
            "--object-mode",
            "record",
            "-o",
            "table",
            "-",
        ])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("alpha.stars  beta.stars  gamma.stars\n          1           2            3\n");
}

#[test]
fn structured_sources_include_late_columns_and_ignore_start_position() {
    let json = fixture(
        "[{\"id\":1,\"name\":\"alpha\"},{\"id\":2,\"name\":\"beta\",\"late\":true}]",
        ".json",
    );
    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table", "--start_pos", "2,2"])
        .arg(json.path())
        .assert()
        .success()
        .stdout("id  name   late\n 1  alpha  \n 2  beta   true\n");

    let ndjson = fixture(
        "{\"id\":1,\"name\":\"alpha\"}\n{\"id\":2,\"name\":\"beta\",\"late\":true}\n",
        ".ndjson",
    );
    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table"])
        .arg(ndjson.path())
        .assert()
        .success()
        .stdout("id  name   late\n 1  alpha  \n 2  beta   true\n");
}

#[test]
fn color_is_plain_by_default_and_opt_in() {
    let file = fixture("A,B\n1,2\n", ".csv");
    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}[").not());

    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table", "--color", "always"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}["));
}

#[test]
fn unsupported_formats_and_colors_fail_during_cli_parsing() {
    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "tui", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'tui'"));

    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["--color", "sometimes", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'sometimes'"));
}

#[test]
fn source_errors_leave_stdout_empty() {
    let file = fixture("[{ broken]", ".json");
    Command::cargo_bin("tabview")
        .expect("binary")
        .args(["-o", "table"])
        .arg(file.path())
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::is_empty().not());
}

#[test]
fn warnings_use_stderr_without_corrupting_table_bytes() {
    let config = tempfile::tempdir().expect("config dir");
    let themes = config.path().join("tabview/themes");
    std::fs::create_dir_all(&themes).expect("themes dir");
    std::fs::write(themes.join("broken.yml"), "name: broken\nstyles: nope\n")
        .expect("broken theme");
    std::fs::write(
        themes.join("also-broken.yml"),
        "name: also-broken\nstyles: nope\n",
    )
    .expect("second broken theme");
    let file = fixture("A,B\n1,2\n", ".csv");

    let output = Command::cargo_bin("tabview")
        .expect("binary")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table"])
        .arg(file.path())
        .output()
        .expect("run tabview");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"A  B\n1  2\n");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_eq!(stderr.matches("theme warning:").count(), 2, "{stderr}");
}

#[test]
fn early_closing_consumer_is_a_clean_exit() {
    let mut contents = String::from("id,value\n");
    for index in 0..100_000 {
        contents.push_str(&format!("{index},row-{index}\n"));
    }
    let file = fixture(&contents, ".csv");
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tabview"))
        .args(["-o", "table"])
        .arg(file.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tabview");
    let mut stdout = std::io::BufReader::new(child.stdout.take().expect("stdout"));
    let mut first_line = String::new();
    stdout.read_line(&mut first_line).expect("first line");
    assert_eq!(first_line.trim(), "id  value");
    drop(stdout);

    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success(), "status: {:?}", output.status);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[cfg(feature = "saved-views")]
#[test]
fn saved_view_controls_non_interactive_projection_and_can_be_disabled() {
    let config = tempfile::tempdir().expect("config dir");
    let views = config.path().join("tabview/views");
    std::fs::create_dir_all(&views).expect("views dir");
    std::fs::write(
        views.join("scripted.yml"),
        r#"
name: scripted
filenames:
  - "*"
columns:
  Name:
    label: NAME
    format: uppercase
  Count:
    type: integer
    width: 4
    align: right
  Extra:
    visible: false
sort:
  - column: Count
    direction: desc
    kind: numeric
filters:
  - column: Count
    action: in
    kind: numeric
    condition: ">2"
"#,
    )
    .expect("saved view");
    let file = fixture(
        "Name,Count,Extra\nalpha,2,x\nbeta,10,y\ngamma,5,z\n",
        ".csv",
    );

    Command::cargo_bin("tabview")
        .expect("binary")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table", "--view", "scripted"])
        .arg(file.path())
        .assert()
        .success()
        .stdout("NAME   Coun\nBETA     10\nGAMMA     5\n")
        .stderr("");

    Command::cargo_bin("tabview")
        .expect("binary")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table", "--no-view"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Extra"))
        .stdout(predicate::str::contains("alpha"));
}

#[cfg(feature = "saved-views")]
#[test]
fn saved_view_warnings_are_emitted_once() {
    let config = tempfile::tempdir().expect("config dir");
    let views = config.path().join("tabview/views");
    std::fs::create_dir_all(&views).expect("views dir");
    std::fs::write(
        views.join("warning.yml"),
        r#"
name: warning
filenames: ["*"]
columns:
  Missing:
    width: 5
"#,
    )
    .expect("saved view");
    let file = fixture("A,B\n1,2\n", ".csv");

    let output = Command::cargo_bin("tabview")
        .expect("binary")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table", "--view", "warning"])
        .arg(file.path())
        .output()
        .expect("run tabview");
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_eq!(
        stderr.matches("saved view: columns.Missing:").count(),
        1,
        "{stderr}"
    );
}
