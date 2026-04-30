// SPDX-License-Identifier: AGPL-3.0-or-later

type RegistryBoard = {
  board_id: string;
  board_dir: string;
  vendor: string;
  fpga_family: string;
  constraint_format: string;
};

type Manifest = {
  boards: string[];
};

type BoardStatusFile = {
  board: string;
  status?: {
    template?: boolean;
    sim?: string;
    synthesis?: string;
    pnr?: string;
    hardware_bringup?: string;
  };
  warnings?: string[];
};

const CONSTRAINT_FILE: Record<string, string> = {
  cst: "pins.cst",
  pcf: "pins.pcf",
  lpf: "constraints.lpf",
  qsf: "project.qsf",
  sdc: "timing.sdc",
  xdc: "constraints.xdc",
  pdc: "constraints.pdc",
};

async function ensureFile(path: string): Promise<void> {
  const st = await Deno.stat(path);
  if (!st.isFile) {
    throw new Error(`missing required file: ${path}`);
  }
}

async function ensureDir(path: string): Promise<void> {
  const st = await Deno.stat(path);
  if (!st.isDirectory) {
    throw new Error(`missing required directory: ${path}`);
  }
}

function requireStatusValue(
  boardId: string,
  key: string,
  value: string | undefined,
  allowed: string[],
): void {
  if (!value || !allowed.includes(value)) {
    throw new Error(
      `invalid board status ${key} for ${boardId}: ${value ?? "<missing>"}`,
    );
  }
}

async function checkBoardStatus(
  path: string,
  boardId: string,
): Promise<void> {
  const statusText = await Deno.readTextFile(path);
  const status = JSON.parse(statusText) as BoardStatusFile;
  if (status.board !== boardId) {
    throw new Error(`board.status.json board mismatch for ${boardId}`);
  }
  if (!status.status || typeof status.status !== "object") {
    throw new Error(`board.status.json missing status object for ${boardId}`);
  }

  requireStatusValue(boardId, "sim", status.status.sim, [
    "not_applicable",
    "not_started",
    "template_only",
    "sim_passed",
  ]);
  requireStatusValue(boardId, "synthesis", status.status.synthesis, [
    "not_measured",
    "not_started",
    "template_only",
    "synth_passed",
  ]);
  requireStatusValue(boardId, "pnr", status.status.pnr, [
    "not_measured",
    "not_started",
    "template_only",
    "pnr_passed",
  ]);
  requireStatusValue(
    boardId,
    "hardware_bringup",
    status.status.hardware_bringup,
    ["not_tested", "not_started", "template_only", "board_tested"],
  );

  if (status.status.template === true) {
    if (
      status.status.sim !== "not_applicable" ||
      status.status.synthesis !== "not_measured" ||
      status.status.pnr !== "not_measured" ||
      status.status.hardware_bringup !== "not_tested"
    ) {
      throw new Error(
        `template board ${boardId} must not claim sim/synthesis/pnr/hardware pass status`,
      );
    }
    if (!Array.isArray(status.warnings) || status.warnings.length === 0) {
      throw new Error(`template board ${boardId} must carry draft warnings`);
    }
  }
}

export async function checkBoards(root = Deno.cwd()): Promise<boolean> {
  const manifestText = await Deno.readTextFile(
    `${root}/cores/af-mod-add/ip.manifest.legacy.json`,
  );
  const manifest = JSON.parse(manifestText) as Manifest;
  if (!Array.isArray(manifest.boards) || manifest.boards.length === 0) {
    throw new Error("manifest boards list is empty");
  }

  const manifestSet = new Set(manifest.boards);
  if (manifestSet.size !== manifest.boards.length) {
    throw new Error("manifest contains duplicate board ids");
  }

  const registryText = await Deno.readTextFile(
    `${root}/registries/boards.registry.json`,
  );
  const registry = JSON.parse(registryText) as { boards: RegistryBoard[] };
  const registrySet = new Set<string>();
  const registryIdSet = new Set<string>();

  for (const entry of registry.boards) {
    if (!registryIdSet.add(entry.board_id)) {
      throw new Error(`duplicate board id in registry: ${entry.board_id}`);
    }

    registrySet.add(entry.board_id);

    const dir = `${root}/${entry.board_dir}`;
    await ensureDir(dir);
    await ensureFile(`${dir}/README.md`);
    await ensureFile(`${dir}/bringup.md`);
    const statusPath = `${dir}/board.status.json`;
    await ensureFile(statusPath);
    await checkBoardStatus(statusPath, entry.board_id);
    await ensureDir(`${dir}/constraints`);
    await ensureFile(`${dir}/constraints/README.md`);

    const constraintFile = CONSTRAINT_FILE[entry.constraint_format];
    if (!constraintFile) {
      throw new Error(
        `unsupported constraint format in registry for ${entry.board_id}: ${entry.constraint_format}`,
      );
    }
    await ensureFile(`${dir}/constraints/${constraintFile}`);

    const hasSystemVerilogTop = await Deno.stat(`${dir}/top/af_board_top.sv`)
      .then((v) => v.isFile)
      .catch(() => false);
    const hasVerilogTop = await Deno.stat(`${dir}/top/af_board_top.v`)
      .then((v) => v.isFile)
      .catch(() => false);
    if (!hasSystemVerilogTop && !hasVerilogTop) {
      throw new Error(`missing top wrapper for board ${entry.board_id}`);
    }

    if (!manifestSet.has(entry.board_id)) {
      throw new Error(
        `registry board '${entry.board_id}' missing in manifest boards list`,
      );
    }
  }

  for (const board of manifest.boards) {
    if (!registrySet.has(board)) {
      throw new Error(
        `manifest board '${board}' missing in boards.registry.json`,
      );
    }
  }

  return true;
}
