// SPDX-License-Identifier: AGPL-3.0-or-later
import { checkManifest } from "./manifest_check.ts";
import { checkDocs } from "./docs_check.ts";
import { checkLicense } from "./license_check.ts";
import { checkBoardMatrix } from "./board_matrix.ts";
import { checkBoards } from "./boards_check.ts";
import { checkToolchains } from "./toolchains_check.ts";
import { checkNoPython } from "./no_python_check.ts";

async function main() {
  await checkNoPython(Deno.cwd());
  await checkManifest(Deno.cwd());
  await checkDocs(Deno.cwd());
  await checkLicense(Deno.cwd());
  await checkBoards(Deno.cwd());
  await checkToolchains(Deno.cwd());
  await checkBoardMatrix(Deno.cwd());
  console.log("repo audit passed");
}

if (import.meta.main) {
  main().catch((e) => {
    console.error(e.message);
    Deno.exit(1);
  });
}
