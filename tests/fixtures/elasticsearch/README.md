# Elasticsearch integration fixture

The fixture pins Elasticsearch 9.1.0 to the client compatibility line used by
the optional adapter. It creates a visible index, a hidden index, a data stream,
multivalued data, runtime and multifield mappings, and a cross-index mapping
conflict.

```sh
docker compose -f tests/fixtures/elasticsearch/docker-compose.yml up -d --wait
tests/fixtures/elasticsearch/setup.sh
TVIEW_ELASTICSEARCH_URL=http://localhost:19200 \
  cargo test --features elasticsearch --test elasticsearch_live -- --ignored
```
