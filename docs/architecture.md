# Architecture

AccelFury is a Rust-first FPGA/IP monorepo with one public CLI binary,
packaged cores, board/toolchain registries, imported template assets, and
focused library crates.

## Crates

- `af-cli`: command parsing, output formatting, and service orchestration.
- `af-manifest`: `af-core.toml` structs, TOML parsing, and manifest validation.
- `af-core`: core-level checks that combine manifest validation and RTL inspection.
- `af-rtl-inspector`: shallow RTL file checks. It does not implement a full SystemVerilog parser.
- `af-security`: path normalization and no-shell command execution.
- `af-backend`: backend traits, command records, build plans, and reports.
- `af-backend-verilator`: Verilator availability, lint argv, and smoke checks.
- `af-backend-fusesoc`: FuseSoC `.core` export.
- `af-backend-litex`: staged LiteX backend capability plus generated wrapper skeleton path.
- `af-backend-yosys`: Yosys syntax/synthesis smoke backend.
- `af-backend-sby`: staged SymbiYosys capability.
- `af-backend-flash`: staged openFPGALoader capability.
- `af-backend-vendor`: staged vendor tool orchestration boundary.
- `af-report`: JSON/Markdown report rendering.
- `af-wrapper-gen`: wrapper target orchestration.
- `af-board-db`: board profile schema and validation.
- `af-field-ref`: finite-field reference arithmetic used by vector generation.
- `af-vectors`: deterministic vector generation for imported arithmetic cores.
- `af-host`: host-side bringup command shell kept as an AGPL-licensed imported tool.
- `af-ci`: CI workflow generation.

## Data Flow

1. `af-cli` reads command-line arguments.
2. Domain services load and validate `af-core.toml`.
3. `af-core` checks declared files and top-level presence.
4. Registry commands validate imported board/toolchain metadata under `registries/`.
5. Backend commands are constructed as argv arrays and executed by `af-security`.
6. Reports collect tool versions, commands, artifacts, warnings, and limitations.
7. Docker/devcontainer/CI jobs provide a repeatable open-source runtime for
   Verilator, FuseSoC, LiteX and Yosys.

## MVP Boundaries

The MVP is manifest-first. It avoids:

- full SystemVerilog parsing;
- CDC/RDC/timing signoff;
- vendor bitstream generation;
- package publishing behavior;
- automatic generation of critical hardware logic.

Generated files are limited to wrappers, manifest/package exports, build
scripts, CI files and reports. Handwritten RTL remains the source of hardware
logic.

## Imported Template Assets

`core-template` snapshot `804bf1e` is imported as data and focused crates, not
as a second product. The canonical public interface remains `af`; legacy
template docs live under `docs/template/`, and `cores/af-mod-add` carries both
the canonical `af-core.toml` and the preserved legacy manifest.
