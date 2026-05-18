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
- `af-backend-icarus`: Icarus Verilog compile/elaboration and VVP simulation.
- `af-backend-fusesoc`: FuseSoC `.core` export.
- `af-backend-litex`: staged LiteX backend capability plus generated wrapper skeleton path.
- `af-backend-yosys`: Yosys syntax/synthesis smoke backend.
- `af-backend-sby`: SymbiYosys availability and declared `.sby` target runs.
- `af-backend-nextpnr`: nextpnr family availability and P&R report planning.
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
template docs live under `docs/template/`, and `examples/af-mod-add` carries both
the canonical `af-core.toml` and the preserved legacy manifest.

## Core Taxonomy

See also [FPGA.chat Backend Roles](fpga-chat-backend.md) for how this taxonomy
maps to the deterministic backend functions consumed by fpga.chat / the online
constructor.

`af` classifies cores along two parallel axes. Neither replaces the other;
they answer different questions.

### Axis 1 — `ProjectClass` (scaffold complexity)

Used by `af project classify` and `af core new --class`. It answers
"how big is the scaffold and which directories must exist?".

| Class                  | Recommended template                  | Required artifacts                                            |
|------------------------|----------------------------------------|---------------------------------------------------------------|
| `simple-portable`      | one-file portable core                | `af-core.toml`, `rtl/`                                        |
| `composite-portable`   | composite core with reusable submodules | `af-core.toml`, `rtl/`, `docs/`, `tests/`                    |
| `complex-vendor-aware` | core with vendor backends             | `af-core.toml`, `af-arch.toml`, `rtl/common/`, `vendor/`, `constructor/` |
| `system-platform`      | system project                        | `af-project.toml`, `cores/`, `platforms/`, `constraints/`     |
| `product-stack`        | product catalog                       | `af-product.toml`, `packages/`, `constructor_catalog/`        |

### Axis 2 — `portability_level` (manifesto U0..U4)

Used by `cores.registry.json`, `af-core.toml` (optional `portability_level`
field), and `af core registry list --portability`. It answers "how
replaceable is this RTL across vendors?".

| Level | Meaning                                                                  |
|-------|--------------------------------------------------------------------------|
| `U0`  | Fully portable RTL. No vendor primitives.                                |
| `U1`  | Portable through inference (e.g. RAM/FIFO inference into vendor blocks). |
| `U2`  | Portable RTL plus thin vendor wrappers (clocking, reset distribution).   |
| `U3`  | Single specification implemented by vendor-specific backends.            |
| `U4`  | Replacement not reasonable; abstraction/wrapper/mock only.               |

### Mapping table

`ProjectClass::portability_levels()` (in `crates/af-complexity`) returns the
levels each scaffold class typically spans. The mapping is informative — a
single core picks one level, not a range — but it constrains which classes
make sense as `U0` cores versus which require `U3`/`U4`.

| `ProjectClass`         | Typical `portability_level` |
|------------------------|-----------------------------|
| `simple-portable`      | `U0`, `U1`                  |
| `composite-portable`   | `U1`, `U2`                  |
| `complex-vendor-aware` | `U2`, `U3`                  |
| `system-platform`      | `U3`                        |
| `product-stack`        | `U4`                        |

### Priority axis

`cores.registry.json` adds a third axis — `priority` (`P0`/`P1`/`P2`) —
that tracks which universal cores AccelFury commits to delivering first.
It is independent of the scaffold class and the portability level.

## Self-Check Targets

`af-selfcheck.toml` is the repository-level regression manifest consumed by
`af self check`. Required targets are public in-tree examples such as
`examples/af-pdm-rx` and `examples/af-mod-add`; optional targets can point at
machine-local standalone projects. This keeps `af` testing itself against real
cores without making a public checkout depend on private local paths.
