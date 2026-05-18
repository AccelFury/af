# TODO - AccelFury `af`

Active lifecycle-tool gaps discovered while hardening generated cores.

- [fpga.chat catalog] AF.TODO.BOARDS-REVISION-CAPTURE — `registries/boards.registry.json`
  has 27 board rows; all of them carry `exact_pinout_status = "draft_placeholder"`
  and none declare a board `revision` field. fpga.chat v1 `BoardRecord` schema
  (`schemas/board-record.schema.json`) makes `revision` a REQUIRED top-level
  string. Until upstream captures revisions, every AccelFury board row is
  deferred at `apps/web/public/data/catalog-deferred.json` with reason
  `revision_missing_from_upstream`. Action requested: add `revision` (and a
  `revision_source_locator`) to each board row, sourced from the official board
  schematic or product page; verify against the documented board revision
  printed on silkscreen or in the schematic title block.
- [fpga.chat catalog] AF.TODO.CORES-OSI-LICENSE — `registries/cores.registry.json`
  and all `examples/*/af-core.toml` files declare
  `license = "AccelFury Source Available License v1.0"`. This is NOT on the
  OSI-approved list, so per `.claude/agents/catalog-curator.md` "License gate"
  the AccelFury cores cannot enter the public fpga.chat catalog v1. Either
  publish AccelFury cores under an OSI-approved license (Apache-2.0, BSD-3-Clause,
  MIT, MPL-2.0, etc.) for the entries intended to be shareable, or leave them
  deferred. fpga.chat will mirror the upstream license verbatim and will not
  paraphrase or upgrade it.

## Recently closed

- AF.TODO.CI-CURRENT-TREE-EVIDENCE-GATE — `docker_ci_cd_evidence` now demotes
  workflow-file-only state to `planned` unless a current-tree run record and a
  `SHA256SUMS` bundle are present in the artifact list. Tests in
  `crates/af-report/src/lib.rs`.
