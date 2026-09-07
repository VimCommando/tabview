# Design

Reuse completed view preparation and serialize positional arrays of displayed
strings. This preserves duplicate labels, hidden columns, and saved formatting.
JSON emits columns and rows; JSONL repeats columns beside each values array.
Escape controls through serde_json and reject forced ANSI. Complete source
preparation before writing; retain existing broken-pipe and failure behavior.
