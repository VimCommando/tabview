#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
tag=${1:?Usage: release-check.sh TAG}
version=$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)
pattern='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
if [[ ! "$tag" =~ $pattern ]] || [ "$tag" != "v$version" ]; then
  echo 'Tag must match the manifest version' >&2
  exit 1
fi
[ "$(git rev-parse "$tag^{commit}")" = "$(git rev-parse HEAD)" ] || { echo 'Tag must point at HEAD' >&2; exit 1; }
lock_version=$(awk -F '"' '/^name = "tview"$/ { found=1; next } found && /^version = / { print $2; exit }' Cargo.lock)
[ "$version" = "$lock_version" ] || { echo 'Lockfile version differs' >&2; exit 1; }
awk -v version="$version" '
  /^## / {
    if (found) exit
    prefix="## [" version "] - "
    date=substr($0, length(prefix)+1)
    if (substr($0, 1, length(prefix)) == prefix && date ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/) { found=1; next }
  }
  found { print; if ($0 ~ /^- /) notable=1 }
  END { if (!found || !notable) { print "Missing dated changelog section with notable changes" > "/dev/stderr"; exit 1 } }
' CHANGELOG.md
