// SPDX-License-Identifier: AGPL-3.0-or-later
const REQUIRED_DOCS = [
  "docs/template/00_overview.md",
  "docs/template/01_architecture.md",
  "docs/template/02_interface.md",
  "docs/template/03_microarchitecture.md",
  "docs/template/04_verification_plan.md",
  "docs/template/05_timing_and_constraints.md",
  "docs/template/06_resource_report.md",
  "docs/template/07_board_targets.md",
  "docs/template/08_release_checklist.md",
  "docs/template/09_portability_rules.md",
  "docs/template/10_commercial_licensing.md",
  "docs/template/11_fpga_family_notes.md",
  "docs/template/12_toolchain_notes.md",
  "docs/template/13_security_and_side_channels.md",
];

export async function checkDocs(root = Deno.cwd()): Promise<boolean> {
  for (const rel of REQUIRED_DOCS) {
    const p = `${root}/${rel}`;
    const stat = await Deno.stat(p).catch(() => null);
    if (!stat || !stat.isFile) {
      throw new Error(`missing documentation file: ${rel}`);
    }
    const text = await Deno.readTextFile(p);
    if (text.trim().length === 0) {
      throw new Error(`empty documentation file: ${rel}`);
    }
  }
  return true;
}
