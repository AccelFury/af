// SPDX-License-Identifier: AGPL-3.0-or-later
import { checkManifest } from "./manifest_check.ts";
import { checkDocs } from "./docs_check.ts";
import { checkLicense } from "./license_check.ts";
import { checkBoards } from "./boards_check.ts";
import { checkToolchains } from "./toolchains_check.ts";
import { checkNoPython } from "./no_python_check.ts";

export async function runAllChecks(root = Deno.cwd()): Promise<{
  manifest: boolean;
  docs: boolean;
  license: boolean;
  boards: boolean;
  toolchains: boolean;
  noPython: boolean;
}> {
  return {
    manifest: await checkManifest(root),
    docs: await checkDocs(root),
    license: await checkLicense(root),
    boards: await checkBoards(root),
    toolchains: await checkToolchains(root),
    noPython: await checkNoPython(root),
  };
}

export type CheckResult = Awaited<ReturnType<typeof runAllChecks>>;
