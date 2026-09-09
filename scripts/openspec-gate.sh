#!/usr/bin/env bash
set -euo pipefail
repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
# The association is mandatory even when the implementation diff has no spec files.
base=${1:?Usage: openspec-gate.sh BASE HEAD IDS_OR_NONE}
head=${2:?Head revision required}
association=${3:?Declare associated change IDs, or none}
case "$association" in
  none) ;;
  *[!a-z0-9,-]*|'') echo "Use comma-separated change IDs or none" >&2; exit 1 ;;
esac
merge_base=$(git merge-base "$base" "$head")
state=$(mktemp -d)
trap 'rm -rf "$state"' EXIT
# The gate inspects committed state only, never archives or edits the source tree.
git archive "$head" openspec | tar -xf - -C "$state"
git diff --name-only --no-renames "$merge_base" "$head" -- openspec/changes > "$state/paths"
awk -F/ '
  $3 == "archive" && NF >= 5 { id=$4; sub(/^[0-9]+-[0-9]+-[0-9]+-/, "", id); print id; next }
  NF >= 4 { print $3 }
' "$state/paths" > "$state/ids"
if [ "$association" != none ]; then printf '%s\n' "$association" | tr ',' '\n' >> "$state/ids"; fi
sort -u "$state/ids" > "$state/selected"
mkdir -p "$state/validation/openspec/changes/archive"
if [ ! -s "$state/selected" ]; then echo "OpenSpec: not applicable"; exit 0; fi
while IFS= read -r id; do
  case "$id" in *[!a-z0-9-]*|'') echo "Invalid change ID: $id" >&2; exit 1 ;; esac
  if [ -d "$state/openspec/changes/$id" ]; then echo "$id: archive the associated change before merge" >&2; exit 1; fi
  archives=("$state"/openspec/changes/archive/????-??-??-"$id")
  if [ "${#archives[@]}" -ne 1 ] || [ ! -d "${archives[0]}" ]; then
    echo "$id: expected exactly one preserved archive" >&2; exit 1
  fi
  archive=${archives[0]}
  if [ ! -s "$archive/proposal.md" ] || [ ! -s "$archive/tasks.md" ]; then
    echo "$id: missing archive artifacts" >&2
    exit 1
  fi
  deltas=("$archive"/specs/*/spec.md)
  if [ ! -f "${deltas[0]}" ]; then
    test -s "$archive/no-spec-deltas.md" || { echo "$id: document why this change has no spec deltas" >&2; exit 1; }
  fi
  cp -R "$archive" "$state/validation/openspec/changes/archive/"
  echo "OpenSpec: $id archived; synchronization requires review"
done < "$state/selected"
# Let OpenSpec validate only the selected archives and all final main specs.
(cd "$state/validation" && OPENSPEC_TELEMETRY=0 openspec validate --archived --no-interactive)
(cd "$state" && OPENSPEC_TELEMETRY=0 openspec validate --specs --strict --no-interactive)
