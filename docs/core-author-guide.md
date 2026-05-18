# Core Author Guide

Create a core directory with:

```text
my-core/
  af-core.toml
  rtl/
  tb/
```

Start with manifest-first declarations. The MVP inspector checks that declared source files exist and that `rtl.top` appears in source text as a module or VHDL entity.

Use `af core new <dir> --name <name>` to start a Verilog-2001 portable IP block.
New base cores should stay in portable Verilog; keep SystemVerilog, vendor
primitives, AXI adapters, PLLs, and board-specific logic in optional wrappers
outside the generic core.

Use `af core new <dir> --name <name> --profile reset-sync` for an atomic reset
synchronizer starter. That profile emits a portable `clk`/`src_rst`/`dst_rst`
core with `STAGES` and `RESET_POLARITY` metadata, without bus, FIFO, RAM, DSP,
PLL, or board-pin logic.

Recommended workflow:

```bash
af manifest validate my-core/af-core.toml
af core check my-core
af core tooling my-core
af core lint my-core --backend native
af core lint my-core --backend verilator
af wrapper generate my-core --target fusesoc
af core report my-core
```

For `verilog` and `verilog-2001` manifests, `af core check` also applies a
portable base-core policy: `default_nettype none` is required, while
top-level ports must use explicit Verilog-2001 ANSI direction and `wire`/`reg`
types. SystemVerilog constructs, common vendor macro markers, hidden PLL
markers, and AXI-only markers are rejected in base RTL. Verilog-2001 modules,
parameters, generate blocks, synchronous logic, explicit clock/reset ports, and
portable inferred RAM/DSP structures are allowed.

Keep `known_limitations` explicit. Reports include these limitations so downstream users do not confuse MVP checks with signoff.

Run `af core tooling my-core --json` during core development to record tool
visibility for formal and package flows. The command probes Boolector, Z3,
Yices, Bitwuzla, cvc5, xmllint, FuseSoC, and Edalize, then writes project
artifacts under `artifacts/openfpga-ci/reports/` and
`artifacts/openfpga-ci/logs/`. These artifacts are evidence that the local or
container environment can see the tools; they are not proof that the core has
complete formal coverage or semantically complete package metadata.

For buyer-grade cores, keep `af-core.toml`, OpenSpec contracts, integration
docs, release claims, and CI reports in sync. Missing backend support or weak
diagnostics should become an `af` code fix or a tracked issue.

## Buyer-ready checklist

Use `af core report <core_dir> --json` as the single source of truth. Each row
in `ReusableCoreMaturity.rows` maps to one item below. A row marked
`supported` counts; `planned` and `blocked` do not.

| Item                                  | Evidence row in `af core report`             |
|---------------------------------------|----------------------------------------------|
| Manifest declared and valid           | `manifest_contract`                          |
| Portable source set                   | `source_portability`                         |
| Open-source backend evidence captured | `open_source_tool_evidence`                  |
| Vendor backend evidence captured      | `vendor_tool_evidence`                       |
| Wrapper / package metadata exported   | `wrapper_package_compatibility`              |
| CI evidence for the current tree      | `docker_ci_cd_evidence` (planned/supported)  |
| Board bring-up artifacts              | `board_hardware_evidence`                    |
| License + commercial terms declared   | `release_support_legal_evidence`             |

In addition, set the manifesto axes in `af-core.toml`:

- `portability_level` — the U0..U4 level the core targets.
- `priority` — when the core is listed in `registries/cores.registry.json`.
- `maturity` — `experimental` until smoke and formal-CDC gates exist;
  `preview` after smoke testing; `beta`/`stable` after multi-vendor evidence;
  `deprecated` when superseded.
- `[[verification_required]]` — one entry per declared verification gate.
  Use `evidence = "..."` once the gate has a committed artifact under the
  core directory.

See [fpga-chat-backend.md](fpga-chat-backend.md) for the mapping from
manifesto vocabulary (Fit Doctor / Core Doctor / Constructor / Report Engine /
Registry Sync) to the underlying `af` commands.

### Language: no unqualified "drop-in replacement"

Do not advertise an `af_*` core as a "drop-in replacement" for a vendor IP
without qualifying it as `behavioral equivalent`, `compatibility wrapper`, or
`after verification`. `af compatibility check` reads each core's
`metadata.description`, `known_limitations`, and `README.md` and emits
`AF_COMPATIBILITY_OVERPROMISING_CLAIM` (warning) when the phrase appears
without one of those qualifiers. The rule is enforced as a soft check so
reviewers see the warning before release rather than after.
