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

fn tabview_command() -> Command {
    let mut command = Command::cargo_bin("tabview").expect("binary");
    command.env(
        "XDG_CONFIG_HOME",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-empty-config"),
    );
    command
}

#[cfg(feature = "sqlite")]
fn sqlite_fixture(statements: &[&str]) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect("sqlite fixture directory");
    let path = directory.path().join("fixture.db");
    let runtime = tokio::runtime::Runtime::new().expect("sqlite runtime");
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().expect("utf8 path"))
            .experimental_generated_columns(true)
            .experimental_without_rowid(true)
            .build()
            .await
            .expect("sqlite database");
        let connection = database.connect().expect("sqlite connection");
        for statement in statements {
            connection
                .execute(statement, ())
                .await
                .expect("sqlite fixture statement");
        }
    });
    (directory, path)
}

#[test]
fn direct_table_and_automatic_redirection_match() {
    let file = fixture("Name,Count\nalpha,2\nbeta,10\n", ".csv");
    let expected = "Name   Count\nalpha      2\nbeta      10\n";

    tabview_command()
        .args(["-o", "table"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(expected)
        .stderr("");

    tabview_command()
        .arg(file.path())
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn stdin_pipeline_uses_data_stream_without_terminal_access() {
    tabview_command()
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

    tabview_command()
        .args(["--format", "json", "-o", "table", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("name   stars\nalpha      1\nbeta       2\ngamma      3\n");

    tabview_command()
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
    tabview_command()
        .args(["-o", "table", "--start_pos", "2,2"])
        .arg(json.path())
        .assert()
        .success()
        .stdout("id  name   late\n 1  alpha  \n 2  beta   true\n");

    let ndjson = fixture(
        "{\"id\":1,\"name\":\"alpha\"}\n{\"id\":2,\"name\":\"beta\",\"late\":true}\n",
        ".ndjson",
    );
    tabview_command()
        .args(["-o", "table"])
        .arg(ndjson.path())
        .assert()
        .success()
        .stdout("id  name   late\n 1  alpha  \n 2  beta   true\n");
}

#[test]
fn color_is_plain_by_default_and_opt_in() {
    let file = fixture("A,B\n1,2\n", ".csv");
    tabview_command()
        .args(["-o", "table"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}[").not());

    tabview_command()
        .args(["-o", "table", "--color", "always"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}["));
}

#[test]
fn unsupported_formats_and_colors_fail_during_cli_parsing() {
    tabview_command()
        .args(["-o", "tui", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'tui'"));

    tabview_command()
        .args(["--color", "sometimes", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'sometimes'"));
}

#[test]
fn source_errors_leave_stdout_empty() {
    let file = fixture("[{ broken]", ".json");
    tabview_command()
        .args(["-o", "table"])
        .arg(file.path())
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::is_empty().not());
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_batch_selects_a_sole_table() {
    let (_directory, path) = sqlite_fixture(&[
        "CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT)",
        "INSERT INTO users VALUES (1, 'Ada')",
        "INSERT INTO users VALUES (2, 'Grace')",
        "INSERT INTO users VALUES (3, 'Linus')",
    ]);

    tabview_command()
        .args(["-o", "table"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("id  name"))
        .stdout(predicate::str::contains("Ada"))
        .stdout(predicate::str::contains("Grace"))
        .stdout(predicate::str::contains("Linus"))
        .stderr("");
}

#[cfg(feature = "sqlite")]
#[test]
fn bundled_sqlite_sample_opens_as_one_thousand_rows() {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample/us-counties.sqlite3");
    let directory = tempfile::tempdir().expect("sample copy directory");
    let path = directory.path().join("us-counties.sqlite3");
    std::fs::copy(source, &path).expect("copy bundled SQLite sample");
    let output = tabview_command()
        .args(["-o", "table"])
        .arg(path)
        .output()
        .expect("open bundled SQLite sample");

    assert!(output.status.success(), "status: {:?}", output.status);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);

    let stdout = String::from_utf8(output.stdout).expect("UTF-8 table output");
    let mut lines = stdout.lines();
    let header = lines.next().expect("table header");
    assert!(header.contains("fips"));
    assert!(header.contains("county_name"));
    assert!(header.contains("net_migration_rate_2020"));
    assert_eq!(lines.count(), 1_000);
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_ambiguous_batch_requires_table_without_emitting_stdout() {
    let (_directory, path) = sqlite_fixture(&[
        "CREATE TABLE users(id INTEGER PRIMARY KEY)",
        "CREATE TABLE events(id INTEGER PRIMARY KEY)",
    ]);

    tabview_command()
        .args(["-o", "table"])
        .arg(&path)
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--table"));

    tabview_command()
        .args(["--table", "events", "-o", "table"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("id"));
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_stdin_and_remote_sources_are_rejected_cleanly() {
    tabview_command()
        .args(["--format", "sqlite", "-o", "table", "-"])
        .write_stdin(b"SQLite format 3\0".as_slice())
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("stdin"));

    tabview_command()
        .args([
            "--format",
            "sqlite",
            "-o",
            "table",
            "libsql://example.turso.io/database",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("remote"));
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

    let output = tabview_command()
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
        .env(
            "XDG_CONFIG_HOME",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-empty-config"),
        )
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
source: {}
view:
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

    tabview_command()
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table", "--view", "scripted"])
        .arg(file.path())
        .assert()
        .success()
        .stdout("NAME   Coun\nBETA     10\nGAMMA     5\n")
        .stderr("");

    tabview_command()
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table", "--no-view"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Extra"))
        .stdout(predicate::str::contains("alpha"));
}

#[cfg(all(feature = "saved-views", feature = "sqlite"))]
#[test]
fn sqlite_saved_source_and_view_layers_apply_in_order() {
    let config = tempfile::tempdir().expect("config dir");
    let views = config.path().join("tabview/views");
    std::fs::create_dir_all(&views).expect("views dir");
    std::fs::write(
        views.join("sqlite.yml"),
        r#"
name: sqlite
filenames: ["*"]
source:
  format: sqlite
  table: events
  limit: 2
  filters:
    - column: active
      operator: equal
      value: true
  sort:
    - column: id
      direction: desc
view:
  filters:
    - column: name
      action: in
      kind: text
      condition: a
  sort:
    - column: name
      direction: asc
      kind: lexical
"#,
    )
    .expect("saved view");
    let (_directory, path) = sqlite_fixture(&[
        "CREATE TABLE events(id INTEGER PRIMARY KEY, name TEXT, active INTEGER)",
        "INSERT INTO events VALUES (1, 'alpha', 1)",
        "INSERT INTO events VALUES (2, 'beta', 0)",
        "INSERT INTO events VALUES (3, 'gamma', 1)",
        "INSERT INTO events VALUES (4, 'delta', 1)",
    ]);

    tabview_command()
        .env("XDG_CONFIG_HOME", config.path())
        .args(["--view", "sqlite", "-o", "table"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("delta"))
        .stdout(predicate::str::contains("gamma"))
        .stdout(predicate::str::contains("alpha").not())
        .stdout(predicate::str::contains("beta").not());
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
source: {}
view:
  columns:
    Missing:
      width: 5
"#,
    )
    .expect("saved view");
    let file = fixture("A,B\n1,2\n", ".csv");

    let output = tabview_command()
        .env("XDG_CONFIG_HOME", config.path())
        .args(["-o", "table", "--view", "warning"])
        .arg(file.path())
        .output()
        .expect("run tabview");
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_eq!(
        stderr.matches("saved view: view.columns.Missing:").count(),
        1,
        "{stderr}"
    );
}
