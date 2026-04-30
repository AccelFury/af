// SPDX-License-Identifier: AGPL-3.0-or-later

const FORBIDDEN_FILENAMES = [
  "requirements.txt",
  "pyproject.toml",
  "setup.py",
  "setup.cfg",
  "tox.ini",
  ".python-version",
  "Pipfile",
  "Pipfile.lock",
  "poetry.lock",
  "package.json",
  "node_modules",
  "requirements-dev.txt",
];

async function walk(
  path: string,
  cb: (file: string) => Promise<void>,
): Promise<void> {
  for await (const entry of Deno.readDir(path)) {
    if (
      entry.name === ".git" || entry.name === "target" ||
      entry.name === "obj_dir"
    ) {
      continue;
    }

    const full = `${path}/${entry.name}`;
    if (FORBIDDEN_FILENAMES.includes(entry.name)) {
      throw new Error(`forbidden file present: ${full}`);
    }

    if (entry.isDirectory) {
      await walk(full, cb);
    } else if (entry.isFile) {
      await cb(full);
    }
  }
}

export async function checkLicense(root = Deno.cwd()): Promise<boolean> {
  await walk(root, async (path) => {
    if (
      path.endsWith(".sv") || path.endsWith(".v") || path.endsWith(".rs") ||
      path.endsWith(".ts")
    ) {
      const text = await Deno.readTextFile(path);
      if (!text.includes("SPDX-License-Identifier")) {
        throw new Error(`missing SPDX header in ${path}`);
      }
    }
  });

  return true;
}
