# CLI Reference

Global flags:

- `--json`: print machine-readable output.
- `--verbose`: increase log verbosity.
- `--quiet`: suppress human output.
- `--build-root <path>`: choose output directory, default `.af-build`.

Commands:

```bash
af doctor
af manifest validate <path>
af core check <core_dir>
af core new <core_dir> --name <name>
af core lint <core_dir> --backend verilator
af core sim <core_dir> --backend verilator
af core report <core_dir_or_build_dir>
af registry check
af board matrix --output docs/board_matrix.md
af board new --board-id <id> --vendor <vendor> --family <family> --constraint-format <format>
af vectors generate
af wrapper generate <core_dir> --target fusesoc
af ci generate --target github-actions
```

Stable exit codes:

- `0`: success.
- `2`: validation or input structure error.
- `3`: backend command failed.
- `4`: backend unavailable.
- `5`: output/report generation failed.

Every CLI error has:

- `code`
- `message`
- `hint`
- `exit_code`
