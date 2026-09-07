#!/usr/bin/env bash
set -euo pipefail
# Authored docs use inline Markdown links; skip fenced examples and external URLs.
for file in docs/*.md; do
  while IFS= read -r link; do
    case "$link" in https://*|http://*|mailto:*|'#'*) continue ;; esac
    path=${link%%#*}
    test -e "$(dirname "$file")/$path" || { echo "$file: broken local link $link" >&2; exit 1; }
  done < <(awk '
    /^```/ { fence = !fence; next }
    !fence { line=$0; while (match(line, /\]\([^ )]+\)/)) {
      print substr(line, RSTART+2, RLENGTH-3); line=substr(line, RSTART+RLENGTH)
    }}' "$file")
done
