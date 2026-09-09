#!/usr/bin/env bash
set -euo pipefail
repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
# shellcheck source=scripts/tools-versions.sh
source scripts/tools-versions.sh
mode=${1:-all}
rust() { rustup run "$RUST_TOOLCHAIN" cargo "$@"; }
docs() {
  test "$(okf --version)" = "okf $OKF_VERSION (OKF spec v0.2)" || { echo "Install okf $OKF_VERSION" >&2; exit 1; }
  okf validate docs/
  bash scripts/docs-links-check.sh
  # All authored concepts must be discoverable from the bundle-root index.
  for file in docs/*.md; do
    case "$file" in docs/index.md|docs/log.md) continue ;; esac
    rg -Fq "($(basename "$file"))" docs/index.md || { echo "Unindexed concept: $file" >&2; exit 1; }
  done
}
shell_checks() {
  shellcheck scripts/*.sh examples/*.sh tests/fixtures/elasticsearch/*.sh
  bash scripts/checks-test.sh
  actionlint .github/workflows/*.yml
}
specs() {
  test "$(OPENSPEC_TELEMETRY=0 openspec --version)" = "$OPENSPEC_VERSION" || { echo "Install OpenSpec $OPENSPEC_VERSION" >&2; exit 1; }
  OPENSPEC_TELEMETRY=0 openspec validate --specs --strict --no-interactive
}
rust_checks() {
  rust fmt --all --check
  rust clippy --locked --all-targets --all-features -- -D warnings
  rust test --locked --quiet
  rust test --locked --no-default-features --quiet
  rust test --locked --all-features --quiet
  for feature in saved-views sqlite elasticsearch; do
    rust clippy --locked --no-default-features --features "$feature" --all-targets --quiet -- -D warnings
  done
}
case "$mode" in
  docs) docs ;;
  rust) rust_checks ;;
  msrv) rustup run "$RUST_MSRV" cargo test --locked --all-features --quiet
        rustup run "$RUST_MSRV" cargo test --locked --no-default-features --quiet ;;
  specs) specs ;;
  scripts) shell_checks ;;
  all) docs; shell_checks; specs; rust_checks ;;
  *) echo "Usage: $0 [all|docs|rust|msrv|specs|scripts]" >&2; exit 2 ;;
esac
