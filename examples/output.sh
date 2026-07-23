#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

tabview() {
    if [ -n "${TABVIEW_BIN:-}" ]; then
        "$TABVIEW_BIN" "$@"
    else
        cargo run --quiet --manifest-path "$repo_root/Cargo.toml" -- "$@"
    fi
}

render() {
    printf 'Command: tabview --output table'
    for argument do
        printf ' %s' "$argument"
    done
    printf '\nReformatted table:\n'
    tabview --output table "$@"
    printf '\n'
}

show_raw_input() {
    echo "Raw input:"
    cat "$1"
    printf '\n\n'
}

# Delimited input with tab-separated columns and Windows line endings.
echo "Delimited input: detects tab-separated columns and Windows line endings"
show_raw_input "sample/windows_newlines.csv"
render "sample/windows_newlines.csv"

# A JSON array of objects.
echo "---"
echo "JSON input: flattens an array of objects into rows and columns"
show_raw_input "sample/json/array-of-objects.json"
render "sample/json/array-of-objects.json"

# Newline-delimited JSON with a column discovered in a later record.
echo "---"
echo "NDJSON input: discovers a column introduced by a later record"
show_raw_input "sample/json/records.ndjson"
render --format ndjson "sample/json/records.ndjson"

# A table nested inside an Elasticsearch-style response.
echo "---"
echo "JSON Pointer: selects and flattens hits from an Elasticsearch response"
show_raw_input "sample/json/elasticsearch-response.json"
render --format json --json-path /hits/hits \
    "sample/json/elasticsearch-response.json"
