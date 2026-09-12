#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
# shellcheck source=scripts/tools-versions.sh
source scripts/tools-versions.sh
rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --component rustfmt,clippy
rustup toolchain install "$RUST_MSRV" --profile minimal
if ! okf --version 2>/dev/null | rg -Fq "okf $OKF_VERSION "; then
  rustup run "$RUST_TOOLCHAIN" cargo install okf --version "$OKF_VERSION" --locked
fi
npm install --global --ignore-scripts "@fission-ai/openspec@$OPENSPEC_VERSION"
actionlint_version() {
  actionlint -version 2>/dev/null | awk 'NR == 1 { print $1; exit }'
}
if ! command -v actionlint >/dev/null 2>&1 || [ "$(actionlint_version)" != "$ACTIONLINT_VERSION" ]; then
  GOBIN="$HOME/.cargo/bin" go install "github.com/rhysd/actionlint/cmd/actionlint@v$ACTIONLINT_VERSION"
fi
