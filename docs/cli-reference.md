# CLI Reference

Global flags:

- `--json`: print machine-readable output.
- `--verbose`: increase log verbosity from default `info` to `debug`; repeat
  for `trace`.
- `--quiet`: suppress human output and lifecycle logs.
- `--color <always|auto|never>`: control ANSI color in human stderr output and
  tracing logs. Default is `always`; JSON stdout is never colorized. Lifecycle
  logs are emitted to stderr by default for every command.
- `--build-root <path>`: choose output directory, default `.af-build`.

Commands:

```bash
af doctor
af self check [--config af-selfcheck.toml] [--include-optional] [--target <name>]
af tooling check
af tooling plan --profile oss --install-mode docker --allow-network
af tooling ensure --profile oss --install-mode docker --allow-network --yes
af tooling ensure --tools fusesoc,edalize,litex --install-mode user --allow-network --yes
af manifest validate <path>
af project classify [path] [--from-spec spec.md] [--output classification.json]
af project new <project_dir> --class system-platform|product-stack [--name <name>]
af core check <core_dir>
af core new <core_dir> --name <name> [--class simple-portable|composite-portable|complex-vendor-aware] [--language verilog-2001] [--profile stream-ip|reset-sync] [--standards-profile fpga-ip-core-v1]
af core lint <core_dir> --backend verilator
af core lint <core_dir> --backend yosys
af core lint <core_dir> --backend icarus
af core sim <core_dir> --backend verilator
af core sim <core_dir> --backend icarus
af core formal <core_dir> --backend sby
af core tooling <core_dir> [--require-all]
af core package <core_dir> --format manifest|tar.zst|spdx-hbom
af core regs scaffold <core_dir> [--output regs/<core>.rdl] [--declare]
af core regs check <core_dir> [--path regs/<core>.rdl]
af core standards check <core_dir> [--profile fpga-ip-core-v1] [--strict]
af core standards doctor [--profile fpga-ip-core-v1]
af core standards drift [--profile fpga-ip-core-v1]
af core standards export --profile fpga-ip-core-v1 --format json|checklist|csv [--output <path>]
af core standards scaffold <core_dir> [--profile fpga-ip-core-v1] [--declare] [--safety-domain none|automotive|industrial|avionics]
af core standards spdx-audit <core_dir> [--output reports/spdx-header-audit.json] [--declare]
af core standards collect <core_dir> --build-root <path> [--profile fpga-ip-core-v1] [--declare]
af core report <core_dir_or_build_dir>
af core verify <core_dir> --tier community|verified-package|enterprise
af architecture check <core_dir>
af resource plan <core_dir> --vendor xilinx --family ultrascale-plus
af resource plan <core_dir> --board tang-nano-20k
af compatibility check <core-a> <core-b>
af compatibility check <system> --constructor
af constructor export <core_or_project> --output .af-build/constructor
af constructor assemble <core_dir>... --board <id> --name <name> --output <dir>
af signoff plan <core_or_project> --class complex-vendor-aware
af dependency graph <core_dir> --format json|dot
af registry check
af board list
af board check <path>
af board matrix --output docs/board_matrix.md
af board new --board-id <id> --vendor <vendor> --family <family> --constraint-format <format>
af build <core_dir> --board <board> --backend litex
af build <core_dir> --board <board> --backend yosys
af build <core_dir> --board <board> --backend nextpnr
af flash <build_dir> --backend openfpgaloader
af clean --yes
af backend list
af backend scaffold <core_dir> --vendor xilinx --family ultrascale-plus
af backend run native --target portable-check --core-dir <core_dir>
af backend run verilator --target lint --core-dir <core_dir>
af backend run icarus --target sim --core-dir <core_dir>
af backend run yosys --target syntax --core-dir <core_dir>
af backend run sby --target formal --core-dir <core_dir>
af backend run nextpnr --target doctor
af evidence ingest --kind simulation-log --input sim.log --tool iverilog --core <core>
af evidence ingest --kind synthesis-report --input synth.json --tool yosys --status passed
af vectors generate
af wrapper generate <core_dir> --target fusesoc
af wrapper generate <core_dir> --target litex --board <board>
af wrapper generate <core_dir> --target ipxact
af wrapper generate <core_dir> --target stream-fifo
af ci init --project <name> --hdl <verilog-2001|verilog-2005> --rtl <path> --top <module> [--sim <cmd>] [--provider github] [--standards --standards-core-dir <path> --standards-profile fpga-ip-core-v1]
af ci render --config af-ci.toml --output .github/workflows/hdl-ci.yml [--dry-run]
af ci doctor --repo .
af ci improve --repo . [--workflow .github/workflows/hdl-ci.yml] [--allow-rewrite] [--dry-run]
af ci add-board --repo . --name <name> --family gowin|ice40|ecp5 --top <module> --device <dev> --constraints <path> --nextpnr-family <family> --pack-device <pkg> [--package <pkg>] [--source-globs <glob,...>] [--dry-run]
af ci validate --repo . [--config af-ci.toml]
af ci run-local --repo . --profile sim|synth|doctor [--dry-run]
af ci generate --target github-actions
af ci generate --target github-actions --backends native,verilator,yosys --optional-fail-closed
af release check [--tag vX.Y.Z] [--ci-evidence <path>] [--artifact-dir <path>] [--docker-evidence <path>] [--output <path>]
af agent kinds
af agent context [--from-error <file.json>]
af agent issue --kind <kind> --title <s> [--summary <s>] [--from-error <file>] [--output <file>]
af agent gh-url --kind <kind> --title <s> --body-file <path> [--labels <l1,l2>]
af agent gh-cli --kind <kind> --title <s> --body-file <path> [--labels <l1,l2>]
See docs/af-ci.md for behavior and docs/af-ci-security.md for policy.
`af ci doctor --json` and `af ci validate --json` include `problem_classes`
for stable automation over workflow, artifact, policy, board, and documentation
contract failures.

`af manifest validate` accepts either a manifest path from the workspace root,
such as `af manifest validate projects/<core>/af-core.toml`, or `af-core.toml`
from inside the core directory. Same-workspace `[[dependencies.cores]]` path
resolution is identical for both invocation forms.

Automation and LLM operators should prefer `--json`, keep generated outputs
under an explicit build root, and avoid inventing unsupported command flags or
signoff claims. Private notes and scratch artifacts belong under ignored
local workspace paths, not in public docs or reports.
```

## Alpha-supported commands

The alpha-readiness surface is the manifest-first development loop:

- `af doctor`
- `af self check`
- `af manifest validate`
- `af core check`
- `af core lint`
- `af core sim`
- `af core report`
- `af core regs`
- `af core standards check`
- `af core standards doctor`
- `af core standards drift`
- `af core standards export`
- `af core standards scaffold`
- `af wrapper generate`
- `af ci generate`
- `af release check`

For these commands, `--json` output, documented exit-code bands, typed
`command_payload` report variants where applicable, and top-level JSON error
envelopes are part of the alpha contract. The error envelope is always:
`{ code, message, hint, exit_code, details? }`.

Commands outside this list are available for development and integration work,
but remain staged or experimental unless their own section states otherwise.
They must still avoid panics and emit structured errors, but their flags,
report details, and backend coverage may change before v1.0.

## Production-supported contract

Production-ready releases promote the same manifest-first command set from
alpha to a stable automation contract. For those commands:

- documented flags and positional arguments are public API;
- `--json` output must remain deterministic and machine-readable;
- the top-level error envelope stays `{ code, message, hint, exit_code,
  details? }`;
- `exit_code` in JSON must match the process exit code;
- `AF_*` error-code removals or meaning changes are breaking changes;
- additive JSON fields are allowed when consumers can ignore them;
- removals or incompatible type changes require a schema/report version bump,
  changelog entry, and migration or backward-compatibility note.

Semver policy before v1.0 is still conservative: production-supported commands
must not break automation inside a patch release. Breaking CLI, manifest, JSON,
schema, or error-code changes require an explicit pre-release or minor-version
promotion note in `CHANGELOG.md`.

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

### Machine-readable JSON contract (M3)

Every report emitted by `af` carries the following deterministic blocks so
LLM/CI consumers can branch on shape without sniffing the schema:

- **`generated_by`** — always the literal `"Generated by AccelFury IP Toolchain"`.
- **`reproducibility`** — `{ host_os, host_arch, environment_hash, af_version }`.
  Deterministic by construction: `environment_hash` is an FNV1a64 hex of the
  sorted `tool=version` list, so identical inputs on identical hosts produce
  byte-identical reports (NFR-004).
- **`command_payload`** — a `#[serde(tag = "kind")]` discriminated union with
  one variant per command family:
  - `check` → `{ manifest_status, source_count, inspection_issue_count,
    legal_issue_count }`
  - `lint` → `{ backend, backend_status, source_count, include_dir_count }`
  - `simulation` → `{ backend, backend_status, testbench_count }`
  - `formal` → `{ backend, backend_status, property_count }`
  - `build` → `{ backend, backend_status, board }`
  - `package` → `{ format, manifest_path }`
  - `report` → `{ input_kind, maturity_verdict, maturity_blocked_rows,
    artifact_count }`
  - `tooling` → `{ total_tools, available_tools, missing_tools }`
  - `doctor` → `{ overall_status, total_tools, available_tools, missing_tools }`
  - `flash` → `{ backend, backend_status }`
  - `release` → `{ target_version, target_tag, commit_sha, readiness_path,
    gate_summary, gates }`

  Consumers should dispatch on `command_payload.kind` and ignore unknown
  kinds for forward compatibility.
- **`standards`** — optional and present only when a core manifest declares
  `[standards]`. It summarizes the selected profile, row counts, limitations,
  and evidence status from `af core standards check` without making
  certification or vendor signoff claims.
- **JSON schema** — the machine-readable schema lives at
  [`schemas/af-report.schema.json`](../schemas/af-report.schema.json). It
  is autogenerated from `crates/af-report::AfReport` (via `schemars`) by
  running `cargo run --quiet --example dump_schema -p af-report > schemas/af-report.schema.json`.
  Regenerate after any change to public types in `af-report`, `af-backend`,
  `af-security`, or `af-manifest`; `.claude/skills/af-cli-contract-guard/check.sh`
  flags missing regenerations.
- **Command log artifacts** — `core lint`/`sim`/`formal`/`doctor` write each
  command's stdout/stderr to sidecar files under `<build_root>/logs/`,
  referenced from the corresponding `CommandRecord.stdout_log` /
  `stderr_log` fields. Each log carries the `Generated by AccelFury IP
  Toolchain` marker as a first comment line.

`af doctor --json` reports optional host-tool readiness in `tool_versions`.
This includes the OSS HDL tools `iverilog`, `vvp`, `verilator`, `yosys`, `sby`,
and SMT solvers `boolector`, `z3`, `yices-smt2`, `bitwuzla`, and `cvc5`, plus
repository support tools such as `deno` and `deno-audit-repo`; the latter checks
that `deno task audit:repo` is discoverable from the current repository without
executing the write-capable audit task.

`af self check --json` reads `af-selfcheck.toml`, runs supported checks against
required in-tree examples, and also checks optional external local projects when
their `af-core.toml` is present. The default manifest tracks `examples/af-pdm-rx`
with source `https://github.com/AccelFury/af-pdm-rx`, `examples/af-mod-add`,
and optional local standalone projects such as `af-mod-add` and
`af-reset-sync`. Missing optional targets are skipped or reported as warnings;
missing or failing required targets return `AF_SELF_CHECK_FAILED`.

`af release check --json` is the fail-closed production gate for publishing
`af` itself. It writes `<build_root>/release/release-readiness.json` and checks
clean source-tree state, local quality gates, external CI evidence for the exact
commit, release binary checksums, Docker image digest/smoke evidence, and
README/docs claim discipline.
By default a blocked gate exits with `AF_RELEASE_READINESS_BLOCKED` (exit 2);
use `--allow-blocked` only when inspecting the readiness report before all
external evidence exists.

`af tooling` is the first-class missing-tool remediation surface. Use
`af tooling check --json` for non-mutating diagnostics, `af tooling plan --json`
to inspect install actions, and `af tooling ensure --yes` to execute approved
actions. The default install mode is `docker`, which keeps heavy OSS EDA tools
such as Icarus Verilog (`iverilog`/`vvp`), Verilator, Yosys, FuseSoC,
Edalize, xmllint, SymbiYosys, Boolector, Z3, Yices, Bitwuzla, cvc5, LiteX, and
openFPGALoader out of the host OS package set by building the Docker runtime
instead. User-local mode is limited to af-managed Python venv installs for
supported Python packages.
System package installation is never implicit; it requires
`--install-mode system --allow-system --allow-network --yes`. `sby` is checked
as a first-class tool, but system installation is reported as manual/container
unless the local distribution provides an intentionally configured package.
Vendor tools such as `gw_sh` and `programmer_cli` are detect-only/manual because
their installers, licenses, and EULAs are outside `af` ownership. If
`af-toolchain.toml` keeps
`offline = true` or `allow_network = false`, networked install actions are
reported as policy-blocked unless the specific run passes `--allow-network`.
Use `docs/vendor-tooling.md` for manual Gowin setup and private Docker bind-mount
guidance.

For a host-level Debian/Ubuntu install of the OSS HDL tools used by the local
flows, run `scripts/pre-install.sh --yes`. It delegates to
`scripts/install-oss-hdl-tools.sh`, `scripts/install-smt-solvers.sh`, and
`scripts/install-core-integration-tools.sh`. Add `.af-tools/python/bin` to
`PATH` before checking Python-package tools from the af-managed virtualenv.
Confirm visibility with
`af tooling check --tools
iverilog,vvp,yosys,nextpnr-ice40,nextpnr-ecp5,nextpnr-gowin,sby,boolector,z3,yices-smt2,bitwuzla,cvc5,verilator,xmllint,fusesoc,edalize
--json`. The HDL script uses apt for `iverilog` (`vvp`), `yosys`, `verilator`,
and available nextpnr packages; if the distribution does not provide an `sby`
package, `--with-sby-source` installs SymbiYosys from its upstream source. The
SMT solver script uses apt where available, an explicit official Yices binary
path for `yices-smt2`, and an explicit Bitwuzla source build path when the
distribution does not package `bitwuzla`. The integration script installs
`xmllint` from `libxml2-utils` and installs FuseSoC/Edalize into the af-managed
Python virtualenv.

`af project classify` emits a deterministic complexity report with
`project_class`, `score`, `triggers`, `recommended_template`,
`required_artifacts`, `warnings`, and `candidate_portability_levels`
(manifesto U0..U4 axis derived from `ProjectClass::portability_levels()`).
It can classify a project directory, manifest, or free-form spec via
`--from-spec`; `--interactive` is accepted as a deterministic first-release
questionnaire placeholder.

`af core new` is the single command for new core scaffolds. If `--class` is
omitted, it defaults to `simple-portable` for compatibility and emits
`AF_COMPLEXITY_CLASS_INFERRED`; run `af project classify` before committing a
complex accelerator template. Supported core classes are `simple-portable`,
`composite-portable`, and `complex-vendor-aware`. Use `--profile reset-sync`
only with `simple-portable` for an atomic reset synchronizer scaffold with
`clk`, `src_rst`, `dst_rst`, `STAGES`, `RESET_POLARITY`, and portable Verilog
policy checks. Use `--standards-profile fpga-ip-core-v1` to opt into the FPGA
IP standards evidence scaffold at creation time. The flag validates the profile
before writing the core, creates the same placeholder files as
`af core standards scaffold --declare`, and records the generated evidence in
`[standards]`. Omitting it preserves the smaller universal scaffold.

`af project new` owns system/product scaffolds. Use `--class system-platform`
for platform projects with cores, platforms, constraints, and security policy
metadata, or `--class product-stack` for constructor catalog/package stacks.

`af architecture check` is a structural offline layer check. It fails on vendor
primitive/PLL/hard-IP markers in `rtl/common`, missing resource contracts for
resource-like RTL, missing CDC contracts for multi-clock manifests, unsupported
backend variants without limitations, and incomplete constructor metadata.

`af resource plan` reads v0.3 resource contracts and emits approximate offline
BRAM/DSP/LUT intent without running vendor tools. `--board` uses the built-in
offline target alias table; exact utilization remains a vendor-report concern.

`af backend scaffold` creates `vendor/<vendor>/backend.toml`, RAM/FIFO/DSP/clock
areas, constraints, an equivalence plan, and unsupported capability notes. It
does not generate fake working RTL; the backend is marked `planned` until real
implementation and evidence exist.

`af constructor export` writes the online-constructor JSON bundle:
`core.json`, `interfaces.json`, `parameters.json`, `compatibility.json`,
`resources.json`, `variants.json`, `dependencies.json`, `reports.json`, and
`limitations.json`.

`af compatibility check`, `af signoff plan`, and `af dependency graph` provide
manifest-level compatibility diagnostics, class-specific signoff matrices, and
reuse graph output for composite and complex projects. Each command first
runs `af_core::load_validated_manifest` on every core directory it is given
(structural manifest + RTL inspection, without legal-policy validation); a
broken manifest or missing source file fails the command with
`AF_CORE_CHECK_FAILED` (exit 2) rather than returning a misleadingly empty
report.

`af project classify --from-spec <path>` refuses an empty or whitespace-only
spec file with `AF_COMPLEXITY_SPEC_EMPTY` (exit 2). An empty spec has no
signal and must not produce a confident classification.

`af board list` prefixes each human-output entry with `[VERIFIED]` or
`[DRAFT]` based on the registry `exact_pinout_status` field
(`verified_on_hardware` is the only `[VERIFIED]` value; everything else
including `draft_placeholder` is `[DRAFT]`). JSON output is unchanged and
continues to expose the raw `exact_pinout_status`.

`af wrapper generate --board <id>` adds a warning to `wrapper.warnings` when
the chosen board is not `verified_on_hardware`, or when the id is not present
in `registries/boards.registry.json`. The wrapper itself is still produced
because skeleton generation is template-only by design; the warning is the
honest signal that the resulting wrapper inherits placeholder pin/clock
metadata.

`af wrapper generate --target stream-fifo` emits a generated ready/valid
adapter around a raw FIFO core described by `[contracts.fifo]`. For
`full_write_policy = "accept_when_full_with_read"`, the generated wrapper uses
`s_ready = !full || (m_ready && m_valid)` so composite shells do not hand-copy
FIFO full/read protocol formulas.

Generic protocol contracts under `[[contracts.protocols]]` are consumed by
`af compatibility check` for adapter hints. `stream-fifo` is the first
generated protocol adapter; reset, width, and CDC adapters remain explicit
suggestions until a dedicated wrapper generator is added.

`af core report <core_dir>` now includes a `maturity` object in JSON and a
Reusable Core Maturity section in Markdown. The verdict separates manifest
contract, source portability, evidence portability, wrapper/package
compatibility, open-source backend evidence, vendor evidence, CI/CD evidence,
board/hardware evidence, release/legal evidence, buyer-grade readiness, and
enterprise-grade readiness using fail-closed statuses such as `supported`,
`planned`, `blocked`, and `not-applicable`. Buyer-grade and enterprise-grade
rows remain `blocked` until the corresponding evidence artifacts are present.
When the manifest declares `[standards]`, the same report includes an additive
`standards` summary in JSON and a Markdown standards evidence section. This
summary is evidence traceability only; it does not claim safety, security,
vendor, or certification signoff.

`af core standards export --profile fpga-ip-core-v1` emits the code-owned FPGA
standards profile as JSON, Markdown checklist, or CSV compliance matrix. The
repository-root `CHECKLIST.md`, `compliance_matrix.csv`, and
`compliance_matrix.json` are generated from this profile; `crates/**` remains
the source of truth for the item list, standard pins, artifact kinds, and JSON
shape.

`af core standards check <core_dir> --json` evaluates a core against the
profile using manifest-declared `[standards]` artifacts plus conventional paths
such as `ipxact/<core>.xml`, `regs/<core>.rdl`,
`security/threat_model.md`, and `hbom/<core>.spdx.json`. Missing `now` rows are
`blocked`; missing `foundation` rows are `planned`. Safety and security rows
are hooks only and never imply certification. Each row includes
`validation_status` and `artifact_validations`; invalid artifacts are fail-closed
and keep the row blocked/planned even when the file exists. `partial` means one
of the row's additive artifact kinds is still missing; known alternatives such
as SPDX-vs-CycloneDX HBOM are treated as one-of groups.
The JSON also includes `gates.commercial_baseline_ready`, which is `passed`
only when every `now` row has acceptable evidence. This gate is a commercial
baseline readiness check, not a safety/security/certification claim.

`af core standards check <core_dir> --strict --json` keeps the same JSON shape
and opportunistically runs external validators for selected artifact kinds. If
`xmllint` is present it checks `ip-xact`; if `peakrdl` is present it checks
`systemrdl`; if those tools are missing the built-in semantic result is kept and
the row records a limitation. A present validator that rejects an artifact still
fail-closes that artifact as `invalid`. `verible-lint` still requires
`verible-verilog-lint` when that artifact kind is selected. The top-level
`tool_availability.tools` array reports the same local PATH probe used by
`af core standards doctor`.

`af core standards doctor --json` checks local availability of tools that can
produce or validate standards evidence: `xmllint`, `peakrdl`,
`verible-verilog-lint`, `verilator`, `sby`, `reuse`, and
`spdx-sbom-generator`. Each row includes install/container/manual hints.
Missing tools are reported for planning; they become blocking only when a
selected evidence producer or validator actually requires them.

`af core standards drift --json` performs an offline freshness check against the
profile snapshot date. It flags fast-moving pins such as SPDX and CWE on shorter
review cadences and records that no network freshness verification was made.

`af core standards scaffold <core_dir> --json` writes missing conventional
evidence placeholders for the profile without overwriting existing files. The
scaffold includes spec/datasheet/test-plan placeholders, IEEE 1685-2022 IP-XACT
metadata, a SystemRDL placeholder, N/A notes for UPF/DFT hooks, security/safety
hook placeholders, CI placeholder, and a deterministic `hbom/<core>.spdx.json`.
These files are starting points only; they do not create certification,
buyer-grade, or security claims.
Add `--declare` to append `[standards]` and `[[standards.artifacts]]` manifest
entries for the generated or already-present conventional evidence files. The
command remains idempotent and does not overwrite local evidence content.
Use `--safety-domain automotive`, `industrial`, or `avionics` to make the
safety manual placeholder domain-specific while still explicitly saying the core
is not certified.

`af core regs scaffold <core_dir> --json` writes a manifest-derived
`regs/<core>.rdl` SystemRDL skeleton and can declare it as standards evidence
with `--declare`. `af core regs check <core_dir> --json` validates the skeleton
semantically enough for the standards gate: it must contain an `addrmap` and at
least one register/field. It does not replace a full `peakrdl` or UVM RAL flow.

`af core standards spdx-audit <core_dir> --json` scans `.v`, `.sv`, `.md`,
`.toml`, and `.rs` files for `SPDX-License-Identifier` headers and writes
`reports/spdx-header-audit.json`. With `--declare`, the report is linked to
checklist item 21. The audit checks presence, not legal compatibility.

`af core standards collect <core_dir> --build-root <path> --json` copies known
CI/build outputs into standards evidence locations. Today this includes
`reports/core-lint.json`, `reports/core-sim.json`, and
`reports/core-formal.json` to `reports/standards/`, plus
`package/<core>.hbom.spdx.json` to `hbom/<core>.spdx.json`; `--declare` links
the copied files into `[[standards.artifacts]]` idempotently.

`af ci init --standards --standards-core-dir <path>` enables a standards CI
preset in `af-ci.toml`. Rendered workflows add a `standards_evidence` job that
requires an `af` binary in PATH, runs native lint and SPDX/HBOM packaging into
the CI build root, performs `spdx-audit --declare`, collects those outputs into
standards artifacts, and finishes with `af core standards check --strict`.
The job does not install tools automatically; use `af core standards doctor`
for local install hints. `examples/standards-ready-core` is the minimal in-tree
reference layout for this flow.

`af core package <core_dir> --format spdx-hbom` writes a deterministic
`*.hbom.spdx.json` provenance document under `--build-root/package/`. The HBOM
lists declared source files plus present standards evidence artifacts, roles,
SHA-256 file checksums, and release provenance fields for the current commit,
tree dirty status, exact tag, and tag-signature verification status. It is not
legal advice and does not replace license review. When a file has an
`SPDX-License-Identifier` header, the HBOM file row carries that identifier;
otherwise it uses `NOASSERTION`.

The `board_hardware_evidence` row enumerates every board declared by the
manifest with an explicit `(draft)` or `(verified-or-unknown)` tag, and adds a
specific limitation listing the placeholder board ids when any declared board
has `exact_pinout_status != "verified_on_hardware"` in the registry.

`af evidence ingest` imports simulator logs, lint transcripts, formal verdicts,
synthesis reports, PnR reports, programming logs, hardware measurement
artifacts, and CI run records into a normalized report under
`.af-build/reports/evidence/` and copies the raw input under
`.af-build/evidence/`. The JSON includes `evidence_status`, stable `signals`, a
deterministic `fingerprint_fnv1a64`, and a `release_gate` object. Ingestion does
not rerun the originating EDA tool and does not claim timing, CDC/RDC,
hardware, or vendor signoff.

The `--kind ci-run` variant accepts a JSON input describing a single workflow
run with required `commit_sha` (7-64 hex characters) and `conclusion` (one of
`success`, `failure`, `cancelled`, `neutral`, `timed_out`, `action_required`,
`skipped`, `stale`), plus optional `workflow_run_url`, `artifact_bundle`, and
`sha256sums`. The ingested report carries the parsed record in a `ci_run`
block under `.af-build/reports/evidence/ci_run_report-*.json`, which
`af core report` reads back to evaluate the `docker_ci_cd_evidence` maturity
row.

`af core report` evaluates `docker_ci_cd_evidence` as `supported` only when
at least one ingested `ci-run` record matches the current `git rev-parse HEAD`
(short prefix or full SHA) and its `conclusion` is `success`. Otherwise the row
is `blocked` with a specific limitation: no workflow file and no records ->
"No CI configuration or CI run evidence was discovered."; workflow file with
no record -> "Workflow file present without an attributable CI run evidence
record."; record present but no current commit SHA available -> "Cannot
determine current commit SHA..."; record(s) present but commit_sha mismatch
-> "CI evidence is stale..."; record matches HEAD but conclusion is not
`success` -> "Recorded CI run for current HEAD concluded as ...".

`af core check` enforces additional portable Verilog checks for manifests with
`rtl.language = "verilog"` or `"verilog-2001"`: `default_nettype none` is
required, every top-level port must use explicit Verilog-2001 ANSI direction
and `wire`/`reg` type, and SystemVerilog constructs, common vendor macro
markers, hidden PLL markers, and AXI-only markers are rejected in base RTL
sources. Keep vendor primitives, AXI adapters, and PLL logic in wrappers outside
the generic core.

`af core tooling <core_dir> --json` is the core-development tool visibility
check. It probes SMT solvers `boolector`, `z3`, `yices-smt2`, `bitwuzla`, and
`cvc5`, plus package/integration tools `xmllint`, `fusesoc`, and `edalize`.
It writes `.af-build/reports/core-tooling.json` plus group reports, and also
writes project-local artifacts:

- `artifacts/openfpga-ci/reports/core-tooling.json`
- `artifacts/openfpga-ci/reports/core-smt-solvers.json`
- `artifacts/openfpga-ci/reports/core-integration-tools.json`
- `artifacts/openfpga-ci/logs/smt-solvers-tool-versions.txt`
- `artifacts/openfpga-ci/logs/integration-tool-versions.txt`

The command returns `warning` when tools are missing unless `--require-all` is
set, in which case missing tools fail with `AF_CORE_TOOLING_MISSING`. These
artifacts prove only local/container visibility; they do not prove formal
coverage, package semantic completeness, or hardware readiness.

Use `af core lint <core_dir> --backend native` or `af backend run native
--target portable-check --core-dir <core_dir>` for the built-in AccelFury
portable-core backend. It executes no external commands and is the default
replacement path for service-backed or third-party structural lint when the
goal is base-core portability rather than simulation or synthesis.

Use the Docker runtime when host tools are missing:

```bash
make smoke
```

## Manifesto vocabulary

The AccelFury manifesto talks about v1 commands by short names. The CLI keeps
its own deterministic names; LLM operators and humans can map between the two
using this table. The functional-role mapping (Fit Doctor / Core Doctor /
Constructor / Report Engine / Registry Sync) lives in
[FPGA.chat Backend Roles](fpga-chat-backend.md).

| Manifesto name | Actual command(s)                                                    |
|----------------|----------------------------------------------------------------------|
| `af init core` | `af core new <dir> --name <name> [--class ... --profile ... --standards-profile ...]` |
| `af check`     | `af core check <core_dir>` + `af architecture check <core_dir>` + `af manifest validate <core_dir>` |
| `af sim`       | `af core sim <core_dir> --backend verilator` or `... --backend icarus` |
| `af synth`     | `af core lint <core_dir> --backend yosys` or `af backend run yosys --target synthesis --core-dir <core_dir>` |
| `af report`    | `af core report <core_or_build>` or `af report <input>`              |
| `af package`   | `af core package <core_dir> [--format manifest|spdx-hbom]`           |
| `af doctor`    | `af doctor`                                                          |

Do not invent the manifesto names as actual subcommands. `af` refuses unknown
commands; the table above is the canonical lookup.

For the role mapping (Fit Doctor / Core Doctor / Constructor / Report Engine /
Registry Sync) see [fpga-chat-backend.md](fpga-chat-backend.md).

## Universal-core registry

`registries/cores.registry.json` tracks `af_*` cores along three axes:

- `priority` — `P0` / `P1` / `P2` (delivery priority).
- `portability_level` — `U0` / `U1` / `U2` / `U3` / `U4` (manifesto axis).
- `maturity` — `experimental` / `preview` / `beta` / `stable` / `deprecated`.

```bash
af registry check --json
af core registry list --json
af core registry list --priority P0
af core registry list --portability U0
```

`af registry check` validates the board registry and the cores registry in
one pass; warnings include `AF_CORES_REGISTRY_REFERENCE_MISSING` for entries
whose `reference_path` is not yet present in-tree.

`af registry check --json` also emits an advisory `catalog_readiness` block for
fpga.chat v1 export readiness. Structural registry validity and catalog
readiness are intentionally separate: a registry can return `status = "passed"`
while `catalog_readiness.status = "blocked"` names publish blockers such as
missing board `revision` / `revision_source_locator` fields or non-OSI core
licenses.

## Agent issue interface (for LLM / AI agents)

`af agent <subcommand>` is an **offline** helper set for automated tools
that drive `af` and need to surface bugs/feature requests back to the
repository. The CLI never POSTs to GitHub and never invokes `gh` — it
only renders artefacts. Submission is the agent's (or human operator's)
explicit action.

```bash
af agent kinds                                                # list supported issue kinds
af agent context [--from-error <file.json>] [--json]          # context bundle: af version, repro, commit SHA, repo
af agent issue --kind <kind> --title <s> [--summary <s>] [--from-error <file>] [--output <file>] [--json]
af agent gh-url --kind <kind> --title <s> --body-file <path> [--labels <l1,l2>] [--json]
af agent gh-cli --kind <kind> --title <s> --body-file <path> [--labels <l1,l2>] [--json]
```

Supported kinds (alias → `.github/ISSUE_TEMPLATE/` file):

| alias | template | default labels |
|---|---|---|
| `bug` | `bug_report.md` | `bug,agent-generated` |
| `feature` | `feature_request.md` | `enhancement,agent-generated` |
| `question` | `question.md` | `question,agent-generated` |
| `board-bringup` | `board_bringup.md` | `hardware,agent-generated` |
| `board-request` | `board_request.md` | `board,agent-generated` |
| `ip-request` | `ip_request.md` | `ip-request,agent-generated` |
| `agent-report` | `agent_report.md` | `agent-generated` |

Every body produced by `af agent issue` carries an `## Agent context`
block with `af_version`, `commit_sha`, `host_os`/`host_arch`,
`environment_hash`, `repo`, `working_dir`, `agent_name` (from
`AF_AGENT_NAME` env var when set), and `automated_submission: true`.

See [docs/agent-workflow.md](agent-workflow.md) for the full workflow
LLM/AI agents must follow.
