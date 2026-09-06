#![cfg(feature = "elasticsearch")]

use assert_cmd::Command;
use predicates::prelude::*;

fn endpoint() -> String {
    std::env::var("TVIEW_ELASTICSEARCH_URL").unwrap_or_else(|_| "http://localhost:19200".to_owned())
}

fn tview() -> Command {
    Command::cargo_bin("tview").unwrap()
}

#[test]
#[ignore = "requires tests/fixtures/elasticsearch"]
fn generated_esql_reads_a_selected_index_without_mutating_it() {
    tview()
        .args([
            "--format",
            "elasticsearch",
            "--table",
            "logs-a",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("boom"))
        .stdout(predicate::str::contains("ok"));
}

#[test]
#[ignore = "requires tests/fixtures/elasticsearch"]
fn complete_user_esql_supports_transforms_and_a_hard_limit() {
    tview()
        .args([
            "--format",
            "elasticsearch",
            "--query",
            "FROM logs-a | STATS count = COUNT(*) BY `log.level` | SORT count DESC",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("count"));
}

#[test]
#[ignore = "requires tests/fixtures/elasticsearch"]
fn aliases_and_data_streams_work_as_explicit_from_targets() {
    tview()
        .args([
            "--format",
            "elasticsearch",
            "--table",
            "events-fixture",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("stream"));
}

#[test]
#[ignore = "requires tests/fixtures/elasticsearch"]
fn discovery_mapping_field_caps_authentication_and_alias_passthrough_are_live() {
    tview()
        .args(["--format", "elasticsearch", &endpoint()])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--table or --query"));

    tview()
        .env("ELASTIC_USERNAME", "fixture-user")
        .env("ELASTIC_PASSWORD", "fixture-password")
        .args([
            "--format",
            "elasticsearch",
            "--table",
            "logs-a",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[\"api\",\"prod\"]"));

    tview()
        .args([
            "--format",
            "elasticsearch",
            "--table",
            "logs-current",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("boom"));
}

#[test]
#[ignore = "requires tests/fixtures/elasticsearch"]
fn live_queries_apply_limits_transforms_and_remain_read_only() {
    let count = || {
        tview()
            .args([
                "--format",
                "elasticsearch",
                "--query",
                "FROM logs-a | STATS total = COUNT(*)",
                "--color",
                "never",
                &endpoint(),
            ])
            .output()
            .unwrap()
    };
    let before = count();
    assert!(before.status.success());

    tview()
        .args([
            "--format",
            "elasticsearch",
            "--query",
            "FROM logs-a | SORT @timestamp DESC | LIMIT 1",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"))
        .stdout(predicate::str::contains("boom").not());

    let after = count();
    assert!(after.status.success());
    assert_eq!(
        before.stdout, after.stdout,
        "viewing did not change documents"
    );
}

#[test]
#[ignore = "requires tests/fixtures/elasticsearch"]
fn live_conflict_partial_and_error_paths_keep_stdout_clean() {
    tview()
        .args([
            "--format",
            "elasticsearch",
            "--query",
            "FROM logs-a | THIS IS NOT ES|QL",
            &endpoint(),
        ])
        .assert()
        .failure()
        .stdout("");

    tview()
        .args([
            "--format",
            "elasticsearch",
            "--table",
            "logs-conflict_*",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("conflicted"));

    tview()
        .args([
            "--format",
            "elasticsearch",
            "--query",
            "FROM logs-a,logs-unavailable | KEEP message",
            "--color",
            "never",
            &endpoint(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("boom"))
        .stderr(predicate::str::contains("partial result"));
}
