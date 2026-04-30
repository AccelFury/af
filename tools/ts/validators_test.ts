// SPDX-License-Identifier: AGPL-3.0-or-later
import { checkBoards } from "./boards_check.ts";
import { checkDocs } from "./docs_check.ts";
import { checkLicense } from "./license_check.ts";
import { checkManifest } from "./manifest_check.ts";
import { checkToolchains } from "./toolchains_check.ts";

Deno.test("read-only repository validators pass", async () => {
  const root = Deno.cwd();
  await checkManifest(root);
  await checkDocs(root);
  await checkLicense(root);
  await checkBoards(root);
  await checkToolchains(root);
});
