#!/usr/bin/env bash
set -eu

endpoint="${TVIEW_ELASTICSEARCH_URL:-http://localhost:19200}"

for target in logs-a logs-conflict_a logs-conflict_b logs-unavailable .hidden-fixture; do
  curl -sS -o /dev/null -X DELETE "$endpoint/$target"
done
curl -sS -o /dev/null -X DELETE "$endpoint/_data_stream/events-fixture"
curl -sS -o /dev/null -X DELETE "$endpoint/_index_template/tview-events"

curl -fsS -X PUT "$endpoint/logs-a" \
  -H 'content-type: application/json' \
  -d '{"mappings":{"properties":{"@timestamp":{"type":"date"},"log.level":{"type":"keyword"},"message":{"type":"text","fields":{"keyword":{"type":"keyword"}}},"tags":{"type":"keyword"},"latency":{"type":"long"},"runtime_source":{"type":"keyword"}},"runtime":{"day":{"type":"keyword","script":{"source":"emit(\"fixture\")"}}}}}'

printf '{"create":{"_index":"logs-a","_id":"one"}}\n{"@timestamp":"2026-01-01T00:00:00Z","log.level":"error","message":"boom","tags":["prod","api"],"latency":42,"runtime_source":"a"}\n{"create":{"_index":"logs-a","_id":"two"}}\n{"@timestamp":"2026-01-02T00:00:00Z","log.level":"info","message":"ok","tags":["prod"],"latency":7,"runtime_source":"b"}\n' |
  curl -fsS -X POST "$endpoint/_bulk?refresh=true" \
    -H 'content-type: application/x-ndjson' --data-binary @-
curl -fsS -X POST "$endpoint/logs-a/_alias/logs-current"

curl -fsS -X PUT "$endpoint/logs-conflict_a" \
  -H 'content-type: application/json' \
  -d '{"mappings":{"properties":{"conflicted":{"type":"long"}}}}'
curl -fsS -X PUT "$endpoint/logs-conflict_b" \
  -H 'content-type: application/json' \
  -d '{"mappings":{"properties":{"conflicted":{"type":"keyword"}}}}'
curl -fsS -X PUT "$endpoint/.hidden-fixture" \
  -H 'content-type: application/json' \
  -d '{"settings":{"index.hidden":true}}'
curl -fsS -X PUT "$endpoint/logs-unavailable?wait_for_active_shards=0" \
  -H 'content-type: application/json' \
  -d '{"settings":{"index.routing.allocation.include._name":"no-such-node"},"mappings":{"properties":{"message":{"type":"keyword"}}}}'

curl -fsS -X PUT "$endpoint/_index_template/tview-events" \
  -H 'content-type: application/json' \
  -d '{"index_patterns":["events-*"],"data_stream":{},"template":{"mappings":{"properties":{"@timestamp":{"type":"date"},"message":{"type":"keyword"}}}}}'
curl -fsS -X POST "$endpoint/events-fixture/_doc?op_type=create&refresh=true" \
  -H 'content-type: application/json' \
  -d '{"@timestamp":"2026-01-03T00:00:00Z","message":"stream"}'
