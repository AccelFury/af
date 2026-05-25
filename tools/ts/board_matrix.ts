// SPDX-License-Identifier: AGPL-3.0-or-later

export async function checkBoardMatrix(root = Deno.cwd()): Promise<boolean> {
  const manifest = JSON.parse(
    await Deno.readTextFile(
      `${root}/examples/af-mod-add/ip.manifest.legacy.json`,
    ),
  );
  const boards: string[] = manifest.boards ?? [];

  const boardRegistry = JSON.parse(
    await Deno.readTextFile(`${root}/registries/boards.registry.json`),
  ) as {
    boards: Array<{
      board_id: string;
      board_dir: string;
      vendor: string;
      fpga_family: string;
      constraint_format: string;
    }>;
  };

  const manifestSet = new Set(boards);
  const rows = [[
    "board_id",
    "vendor",
    "family",
    "constraint_format",
    "in_manifest",
    "synthesis",
    "pnr",
    "hardware_bringup",
  ]];

  for (const meta of boardRegistry.boards) {
    const inManifest = manifestSet.has(meta.board_id) ? "yes" : "no";

    const statusPath = `${root}/${meta.board_dir}/board.status.json`;
    let synthesis = "not_measured";
    let pnr = "not_measured";
    let hw = "not_tested";

    try {
      const statusPayload = JSON.parse(await Deno.readTextFile(statusPath)) as {
        status?: {
          synthesis?: string;
          pnr?: string;
          hardware_bringup?: string;
        };
      };
      synthesis = statusPayload.status?.synthesis ?? "not_measured";
      pnr = statusPayload.status?.pnr ?? "not_measured";
      hw = statusPayload.status?.hardware_bringup ?? "not_tested";
    } catch {
      synthesis = "missing";
      pnr = "missing";
      hw = "missing";
    }

    rows.push([
      meta.board_id,
      meta.vendor,
      meta.fpga_family,
      meta.constraint_format,
      inManifest,
      synthesis,
      pnr,
      hw,
    ]);
  }

  await Deno.writeTextFile(
    `${root}/docs/board_matrix.md`,
    `${formatMarkdownTable(rows)}\n`,
  );
  console.log("board matrix written to docs/board_matrix.md");
  return true;
}

function formatMarkdownTable(rows: string[][]): string {
  const widths = rows[0].map((_, column) =>
    Math.max(...rows.map((row) => row[column].length))
  );
  const formatRow = (row: string[]) =>
    `| ${row.map((cell, column) => cell.padEnd(widths[column])).join(" | ")} |`;
  const separator = `| ${
    widths.map((width) => "-".repeat(width)).join(" | ")
  } |`;
  return [formatRow(rows[0]), separator, ...rows.slice(1).map(formatRow)].join(
    "\n",
  );
}
