// SPDX-License-Identifier: AGPL-3.0-or-later

export async function checkManifest(root = Deno.cwd()): Promise<boolean> {
  const coreRoot = `${root}/examples/af-mod-add`;
  const path = `${coreRoot}/ip.manifest.legacy.json`;
  const manifestText = await Deno.readTextFile(path);
  const manifest = JSON.parse(manifestText);
  if (!manifest || typeof manifest !== "object") {
    throw new Error("invalid manifest");
  }
  const required = [
    "schema_version",
    "ip",
    "interfaces",
    "parameters",
    "source_files",
    "rtl_language_files",
    "testbenches",
    "vectors",
    "boards",
    "tooling",
  ];
  for (const key of required) {
    if (!(key in manifest)) {
      throw new Error(`missing manifest key: ${key}`);
    }
  }
  const ip = manifest.ip;
  if (
    !ip || typeof ip.name !== "string" ||
    typeof ip.default_modulus_hex !== "string"
  ) {
    throw new Error("invalid ip metadata");
  }
  if (!Array.isArray(ip.rtl_languages) || ip.rtl_languages.length === 0) {
    throw new Error("manifest.ip.rtl_languages must be a non-empty array");
  }
  const supportedLangs = ["verilog-2001", "verilog-2005", "systemverilog"];
  const actualLangs = new Set(ip.rtl_languages);
  if (actualLangs.size !== supportedLangs.length) {
    throw new Error(
      "manifest.ip.rtl_languages must contain each supported RTL language exactly once",
    );
  }
  for (const lang of ip.rtl_languages) {
    if (!supportedLangs.includes(lang)) {
      throw new Error(
        `unsupported rtl language in manifest.ip.rtl_languages: ${lang}`,
      );
    }
  }
  for (const lang of supportedLangs) {
    if (!actualLangs.has(lang)) {
      throw new Error(`missing required rtl language in manifest: ${lang}`);
    }
  }
  if (typeof ip.rtl_language === "string") {
    if (!supportedLangs.includes(ip.rtl_language)) {
      throw new Error(
        `unsupported manifest.ip.rtl_language: ${ip.rtl_language}`,
      );
    }
    if (!ip.rtl_languages.includes(ip.rtl_language)) {
      throw new Error(
        "manifest.ip.rtl_language must be included in manifest.ip.rtl_languages",
      );
    }
  }
  const languageFiles = manifest.rtl_language_files;
  if (!languageFiles || typeof languageFiles !== "object") {
    throw new Error("manifest.rtl_language_files is required");
  }
  for (const lang of ip.rtl_languages) {
    const files = languageFiles[lang];
    if (!Array.isArray(files) || files.length === 0) {
      throw new Error(
        `manifest.rtl_language_files.${lang} must be a non-empty array`,
      );
    }
    for (const rel of files) {
      const stat = await Deno.stat(`${coreRoot}/${rel}`).catch(() => null);
      if (!stat || !stat.isFile) {
        throw new Error(`missing RTL language file for ${lang}: ${rel}`);
      }
    }
  }
  if (!Array.isArray(manifest.boards) || manifest.boards.length === 0) {
    throw new Error("boards must be non-empty");
  }
  const boards = manifest.boards as string[];
  const boardSet = new Set(boards);
  if (boardSet.size !== boards.length) {
    throw new Error("boards list contains duplicates");
  }
  const toolchain = manifest.tooling;
  for (
    const key of [
      "rust",
      "typescript_deno",
      "python",
      "cocotb",
      "fusesoc_required",
    ]
  ) {
    if (!(key in toolchain)) {
      throw new Error(`missing manifest tooling key: ${key}`);
    }
  }
  if (toolchain.python !== false) {
    throw new Error("manifest tooling.python must be false for this template");
  }
  if (toolchain.cocotb !== false) {
    throw new Error("manifest tooling.cocotb must be false for this template");
  }
  if (toolchain.fusesoc_required !== false) {
    throw new Error(
      "manifest tooling.fusesoc_required must be false for this template",
    );
  }
  const sourceFiles = manifest.source_files;
  if (!sourceFiles.core?.length || !Array.isArray(sourceFiles.core)) {
    throw new Error("manifest.source_files.core must be a non-empty array");
  }
  const registryJson = JSON.parse(
    await Deno.readTextFile(`${root}/registries/boards.registry.json`),
  ) as { boards: Array<{ board_id: string }> };
  const boardRegistry = registryJson.boards ?? [];
  for (const board of boards) {
    if (
      !boardRegistry.some((item: { board_id: string }) =>
        item.board_id === board
      )
    ) {
      throw new Error(
        `manifest board ${board} not found in registries/boards.registry.json`,
      );
    }
  }
  return true;
}
