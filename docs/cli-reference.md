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
af core new <core_dir> --name <name> [--language systemverilog|verilog]
af core lint <core_dir> --backend verilator
af core lint <core_dir> --backend yosys
af core sim <core_dir> --backend verilator
af core formal <core_dir> --backend sby
af core package <core_dir> --format manifest
af core report <core_dir_or_build_dir>
af registry check
af board list
af board check <path>
af board matrix --output docs/board_matrix.md
af board new --board-id <id> --vendor <vendor> --family <family> --constraint-format <format>
af build <core_dir> --board <board> --backend litex
af build <core_dir> --board <board> --backend yosys
af flash <build_dir> --backend openfpgaloader
af clean --yes
af backend list
af backend run verilator --target lint --core-dir <core_dir>
af backend run yosys --target syntax --core-dir <core_dir>
af vectors generate
af wrapper generate <core_dir> --target fusesoc
af wrapper generate <core_dir> --target litex --board <board>
af ci generate --target github-actions
```

Stable exit codes:

- `0`: success.
- `1`: generic error.
- `2`: validation or input structure error.
- `3`: RTL inspection or backend orchestration error.
- `4`: backend unavailable.
- `5`: output/report generation failed.
- `6`: simulation failed.
- `7`: lint failed.
- `8`: formal failed.
- `9`: build failed.
- `10`: flash failed.
- `11`: security policy violation.
- `12`: artifact/report missing.

Every CLI error has:

- `code`
- `message`
- `hint`
- `exit_code`

`af core new` defaults to SystemVerilog and can also emit a Verilog-2001
starter. Use the Verilog starter when a buyer-facing core must remain portable
across older FPGA flows before vendor wrappers are added.

Use the Docker runtime when host tools are missing:

```bash
make smoke
```
