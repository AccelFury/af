# `af` as deterministic backend for fpga.chat / online constructor

The AccelFury manifesto names five functional roles that an LLM-driven front-end
(fpga.chat / online constructor) delegates to `af`. None of those names appears
verbatim in the CLI; they are _roles_, not subcommands. This document is the
canonical mapping from each role to the existing `af` surface so the LLM cannot
invent commands or schemas.

## Fit Doctor

Question: "does this core or set of cores fit a given FPGA?"

| Step                               | Command                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------- |
| Offline resource intent            | `af resource plan <core_dir> --board <board>`                                         |
| Offline resource intent per family | `af resource plan <core_dir> --vendor <vendor> --family <family>`                     |
| Cross-core / system-level fit      | `af compatibility check <core-a> <core-b>` (use `--constructor` for system manifests) |
| Layer leakage check                | `af architecture check <core_dir>`                                                    |

Fit Doctor never claims actual utilization — exact fit requires a vendor report.
The JSON output marks rows as `planned` or `blocked` when the underlying
evidence is missing.

## Core Doctor

Question: "what is wrong with this core's manifest, RTL structure, reset/clock
boundaries, tests, or documentation?"

| Step                          | Command                            |
| ----------------------------- | ---------------------------------- |
| Manifest schema + portability | `af core check <core_dir>`         |
| Manifest field validation     | `af manifest validate <core_dir>`  |
| Layer + CDC + verification    | `af architecture check <core_dir>` |
| Tool readiness for this core  | `af core tooling <core_dir>`       |

`af core check` runs the RTL inspector (see `crates/af-rtl-inspector`) and fails
on portable-RTL policy violations: `AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT`,
`AF_PORTABLE_VENDOR_OR_CLOCK_MARKER`, `AF_PORTABLE_AXI_ONLY_MARKER`,
`AF_PORTABLE_IMPLICIT_RESET`, `AF_PORTABLE_ENCRYPTED_NETLIST`,
`AF_PORTABLE_PORT_STYLE`, `AF_PORTABLE_DEFAULT_NETTYPE_MISSING`.

## Constructor

Question: "assemble a project from cores, wrappers, board manifests, and
constraints."

| Step                               | Command                                                         |
| ---------------------------------- | --------------------------------------------------------------- |
| Constructor metadata export        | `af constructor export <core_or_project>`                       |
| Wrapper generation per target      | `af wrapper generate <core_dir> --target fusesoc`               |
| Wrapper generation per board       | `af wrapper generate <core_dir> --target litex --board <board>` |
| Backend scaffolding (vendor stubs) | `af backend scaffold <core_dir> --vendor <v> --family <f>`      |
| Build orchestration                | `af build <core_dir> --board <board> --backend <b>`             |

The MVP Constructor is _export-side only_: it emits the metadata an online
constructor needs to combine cores. Bidirectional assembly is planned and not
implemented.

## Report Engine

Question: "produce a machine-readable evidence report with confidence, warnings,
limitations, repro steps, and next actions."

| Step               | Command                          |
| ------------------ | -------------------------------- |
| Backend report     | `af report <input>`              |
| Core-scoped report | `af core report <core_or_build>` |

The JSON schema is `crates/af-report/src/lib.rs::AfReport`. The
`ReusableCoreMaturity` block carries evidence rows for `manifest_contract`,
`source_portability`, `evidence_portability`, `wrapper_package_compatibility`,
`open_source_tool_evidence`, `vendor_tool_evidence`, `docker_ci_cd_evidence`,
`board_hardware_evidence`, `release_support_legal_evidence`,
`buyer_grade_readiness`, and `enterprise_grade_readiness`.

`docker_ci_cd_evidence` is gated against current-tree state: a workflow file
alone is `planned`; `supported` requires a current-tree run record plus a
`SHA256SUMS` bundle.

The set of rows that must reach `supported` for a commercial claim is the
contract surface for the `verified-package` / `enterprise` tiers — see
[docs/licensing.md — Commercial tiers](licensing.md#commercial-tiers).

## Registry Sync

Question: "what universal cores exist, at which priority and portability level,
and which have in-tree manifests?"

| Step                       | Command                                  |
| -------------------------- | ---------------------------------------- |
| Validate registries        | `af registry check`                      |
| List universal cores (all) | `af core registry list`                  |
| List by priority           | `af core registry list --priority P0`    |
| List by portability        | `af core registry list --portability U0` |

`registries/cores.registry.json` is read-only from `af`. Bidirectional sync
(GitHub Actions / fpga.camp upload) is out of scope for v1.

## What is NOT a Constructor / Doctor role

- Timing closure. `af` never claims this. Vendor reports are evidence; `af`
  ingests them, it does not produce them.
- CDC/RDC sign-off. Simulation- and inspector-level CDC checks are advisory. Use
  `verification_required` with kind `formal-cdc-assumption` to declare the
  obligation.
- Bitstream production. `af flash` orchestrates `openFPGALoader` and similar, it
  does not own vendor implementation flows.
- LLM-authored facts. The role of fpga.chat is to _explain_ and _route_; the
  ground truth is the JSON output of `af`, manifests, and vendor artifacts.
