# JSON view export and CLI identification

## Why

Fixed-width table output does not provide stable machine-readable framing.
The CLI also lacks a package version flag for installation checks.

## What changes

Add explicit JSON and JSONL serializers for displayed view values, preserve the
automatic table default, and expose the manifest version through `--version`.

## Impact

CLI output, serialization adapters, compatibility tests, and output schemas.
No source-native type round trip or default-mode change is promised.
