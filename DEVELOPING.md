# Development

Tview is maintained as one Rust package. The contributor contract and local
standards-bundle pointer are in [development policy](docs/development.md).

Run the same preflight used by CI:

```bash
bash scripts/check.sh
```

For documentation-only changes, use `bash scripts/check.sh docs`.
For the separate minimum-compiler check, use `bash scripts/check.sh msrv`.
Install pinned tools using the development policy instructions.

Every PR declares its OpenSpec association. Associated changes must be synced
and archived before merge. See the [completion gate](docs/development.md#openspec-completion).

Terminal dialogs follow [modal style](docs/modal-style.md).
Release work follows the [release process](docs/releases.md).
