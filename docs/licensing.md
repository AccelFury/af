# Licensing

The merged monorepo preserves source licensing instead of relicensing imported
surfaces.

- Existing Apache-licensed `af` Rust crates keep `Apache-2.0` via workspace
  package metadata.
- Imported Rust and TypeScript tooling from `core-template` keeps
  `AGPL-3.0-or-later`.
- RTL and gateware under imported core/template surfaces keep `CERN-OHL-S-2.0`.
- Imported documentation keeps `CC-BY-SA-4.0` where applicable.
- Full license texts are stored in `LICENSES/`.

Do not infer a single-project relicensing from file location alone. SPDX headers
and package metadata are the source of truth for individual files.

## Generated AccelFury IP Cores

`af core new` generates reusable FPGA/ASIC IP core scaffolds under
`AccelFury Source Available License v1.0`. Generated cores include `LICENSE`,
`COMMERCIAL-LICENSE.md`, and `NOTICE` files. Closed-source, proprietary,
customer-delivered, ASIC/SoC/chiplet, paid, private, or commercial use requires
a separate paid commercial license from AccelFury.

`af core check` is intentionally fail-closed for generated reusable cores: it
rejects missing legal files, placeholder legal text, or `af-core.toml` metadata
that does not match the approved AccelFury source-available policy. This does
not relicense the Rust implementation of the `af` tool itself.

## Commercial tiers

AccelFury distributes cores in three commercial tiers. Each tier maps to a
specific evidence bar in the `af core report` output (see
[FPGA.chat Backend Roles](fpga-chat-backend.md#report-engine) and
[Core Author Guide — Buyer-ready checklist](core-author-guide.md#buyer-ready-checklist)).
The tier is not encoded in `af-core.toml` directly; it is a contractual
statement from AccelFury, backed by the JSON evidence rows below.

| Tier               | License                          | Evidence bar (`ReusableCoreMaturity.rows`)                                                                                    | Support                             |
| ------------------ | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| `community`        | ASAL v1.0 (public)               | `manifest_contract`, `source_portability` supported. Other rows may be `planned`/`blocked`.                                   | Community, no SLA.                  |
| `verified-package` | ASAL v1.0 + AccelFury audit      | Above + `open_source_tool_evidence`, `wrapper_package_compatibility`, `docker_ci_cd_evidence` (current-tree) all `supported`. | Commercial support on request.      |
| `enterprise`       | Separate paid commercial license | Above + `vendor_tool_evidence`, `board_hardware_evidence`, `release_support_legal_evidence` all `supported`.                  | Custom integration, warranty, SLAs. |

Use `af core verify --tier <tier> <core_dir> --json` to check tier eligibility
automatically. The command computes the same evidence rows as `af core report`
and exits with code 2 (`AF_TIER_REQUIREMENTS_UNMET`) plus a `missing` list when
any required row is not `supported`. A `verified-package` tier statement
requires the corresponding `af core report` JSON output to be archived alongside
the release artefacts. `community` cores may not advertise themselves as
`verified-package` without that archived evidence. The `docker_ci_cd_evidence`
row is gated against the current tree (SHA256SUMS + run record); a workflow file
alone keeps the row at `planned`.

For closed-source / commercial use beyond `community`, contact AccelFury via the
channels listed in [COMMERCIAL.md](../COMMERCIAL.md). The exception covered by
ASAL v1.0 stays in effect for public open-source projects with attribution.
