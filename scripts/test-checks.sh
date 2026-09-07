#!/usr/bin/env bash
set -euo pipefail
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
suite=$(mktemp -d)
trap 'rm -rf "$suite"' EXIT
export GIT_CONFIG_NOSYSTEM=1
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_AUTHOR_NAME=Repository-tests GIT_COMMITTER_NAME=Repository-tests
export GIT_AUTHOR_EMAIL=test@example.invalid GIT_COMMITTER_EMAIL=test@example.invalid
new_repo() {
  repo="$suite/$1"
  mkdir -p "$repo/scripts" "$repo/openspec/specs/example" "$repo/openspec/changes/unrelated"
  cp "$root/scripts/openspec-gate.sh" "$repo/scripts/"
  git -C "$repo" init -q
  cat > "$repo/openspec/specs/example/spec.md" <<'SPEC'
## Purpose
Describe the fixture behavior used to verify the repository completion gate.

## Requirements

### Requirement: Existing
The system SHALL retain existing behavior.

#### Scenario: Existing input
- **WHEN** an existing input arrives
- **THEN** it is accepted
SPEC
  printf 'Unrelated draft\n' > "$repo/openspec/changes/unrelated/proposal.md"
  commit
  base=$(git -C "$repo" rev-parse HEAD)
}
commit() { git -C "$repo" add .; git -C "$repo" -c commit.gpgsign=false commit -qm fixture; }
archive() {
  target="$repo/openspec/changes/archive/2026-09-06-$1"
  mkdir -p "$target/specs/example"
  printf '# Fixture proposal\n' > "$target/proposal.md"
  printf '%s\n' '- [x] Complete fixture' > "$target/tasks.md"
  cat > "$target/specs/example/spec.md" <<'SPEC'
## ADDED Requirements

### Requirement: Added
The system SHALL accept the new input.

#### Scenario: New input
- **WHEN** the new input arrives
- **THEN** it is accepted
SPEC
}
expect_gate() {
  expected=$1; ids=$2
  status=0
  bash "$repo/scripts/openspec-gate.sh" "$base" HEAD "$ids" > "$suite/result" 2>&1 || status=$?
  if { [ "$expected" = pass ] && [ "$status" -ne 0 ]; } || { [ "$expected" = fail ] && [ "$status" -eq 0 ]; }; then
    cat "$suite/result" >&2
    echo "Unexpected gate result: $repo $expected" >&2; exit 1
  fi
}
new_repo unrelated
expect_gate pass none
new_repo edited
mkdir -p "$repo/openspec/changes/edited"
printf 'First\n' > "$repo/openspec/changes/edited/proposal.md"
commit; base=$(git -C "$repo" rev-parse HEAD)
printf 'Edited\n' >> "$repo/openspec/changes/edited/proposal.md"
commit; expect_gate fail none
new_repo deleted
mkdir -p "$repo/openspec/changes/deleted"
printf 'Draft\n' > "$repo/openspec/changes/deleted/proposal.md"
commit; base=$(git -C "$repo" rev-parse HEAD)
rm "$repo/openspec/changes/deleted/proposal.md"
commit; expect_gate fail none
new_repo rename
mkdir -p "$repo/openspec/changes/old"
printf 'Draft\n' > "$repo/openspec/changes/old/proposal.md"
commit; base=$(git -C "$repo" rev-parse HEAD)
mv "$repo/openspec/changes/old" "$repo/openspec/changes/new"
commit; expect_gate fail none
new_repo incomplete
archive feature
printf '%s\n' '- [ ] Unfinished task' >> "$target/tasks.md"
commit; expect_gate fail none
new_repo synced
archive feature
sed '1d' "$target/specs/example/spec.md" >> "$repo/openspec/specs/example/spec.md"
commit; expect_gate pass none
# Explicit association must work when the PR has no spec diff.
base=$(git -C "$repo" rev-parse HEAD)
printf 'Implementation\n' > "$repo/code.txt"
commit; expect_gate pass feature
expect_gate fail feature,missing
expect_gate fail unrelated
# Historical deltas do not freeze the current requirement text.
printf '\n#### Scenario: Later behavior\n- **WHEN** extra input arrives\n- **THEN** it is accepted\n' >> "$repo/openspec/specs/example/spec.md"
commit; expect_gate pass feature
# Release gates reject unpublished history, mismatched tags, and stale lockfiles.
new_repo release
cp "$root/scripts/check-release.sh" "$repo/scripts/"
printf '[package]\nname = "tview"\nversion = "0.1.0"\n' > "$repo/Cargo.toml"
printf '[[package]]\nname = "tview"\nversion = "0.1.0"\n' > "$repo/Cargo.lock"
printf '# Changelog\n\n## [Unreleased]\n- Pending.\n' > "$repo/CHANGELOG.md"
commit; git -C "$repo" tag v0.1.0
if bash "$repo/scripts/check-release.sh" v0.1.0 > "$suite/result" 2>&1; then echo 'Unreleased history accepted' >&2; exit 1; fi
printf '\n## [0.1.0] - 2026-09-06\n\n### Added\n- Initial release.\n' >> "$repo/CHANGELOG.md"
commit
if bash "$repo/scripts/check-release.sh" v0.1.0 > "$suite/result" 2>&1; then echo 'Stale tag accepted' >&2; exit 1; fi
# Replacing a tag is restricted to this temporary fixture repository.
git -C "$repo" tag -f v0.1.0 > /dev/null
bash "$repo/scripts/check-release.sh" v0.1.0 > "$suite/result"
if bash "$repo/scripts/check-release.sh" v0.2.0 > "$suite/result" 2>&1; then echo 'Mismatched tag accepted' >&2; exit 1; fi
printf '[[package]]\nname = "tview"\nversion = "0.2.0"\n' > "$repo/Cargo.lock"
if bash "$repo/scripts/check-release.sh" v0.1.0 > "$suite/result" 2>&1; then echo 'Stale lockfile accepted' >&2; exit 1; fi
echo 'Repository checks: 10 OpenSpec and 5 release cases passed'
