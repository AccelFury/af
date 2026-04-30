// SPDX-License-Identifier: AGPL-3.0-or-later

type ToolchainEntry = {
  id: string;
  name: string;
  display_name?: string;
  vendor?: string;
  command?: string;
};

export async function checkToolchains(root = Deno.cwd()): Promise<boolean> {
  const registryJson = JSON.parse(
    await Deno.readTextFile(`${root}/registries/toolchains.registry.json`),
  ) as { toolchains: ToolchainEntry[] };
  const toolchains = registryJson.toolchains ?? [];

  if (!Array.isArray(toolchains) || toolchains.length < 10) {
    throw new Error("toolchains registry is missing baseline entries");
  }

  const required = [
    "gowin_eda",
    "intel_quartus",
    "xilinx_vivado",
    "lattice_oss_yosys_nextpnr_ice40",
    "lattice_oss_yosys_nextpnr_ecp5",
    "lattice_radiant",
    "efinix_efinity",
    "microchip_libero",
    "verilator",
    "deno",
    "rust_cargo",
  ];

  const present = new Set(toolchains.map((item) => item.id));
  for (const item of required) {
    if (!present.has(item)) {
      throw new Error(`missing required toolchain ${item}`);
    }
  }

  for (const t of toolchains) {
    if (!(t.name || t.display_name) || !t.id) {
      throw new Error(`invalid toolchain entry for ${JSON.stringify(t)}`);
    }
    if (!t.command) {
      throw new Error(`toolchain ${t.id} has missing command`);
    }
  }

  return true;
}
