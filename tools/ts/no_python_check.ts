// SPDX-License-Identifier: AGPL-3.0-or-later

const FORBIDDEN_NAMES = [
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

const FORBIDDEN_NAME_SET = new Set(
  FORBIDDEN_NAMES.map((name) => name.toLowerCase()),
);

const FORBIDDEN_EXTENSIONS = [
  ".py",
  ".pyc",
  ".pyo",
  ".pyd",
  ".c",
  ".cc",
  ".cpp",
  ".cxx",
  ".h",
  ".hh",
  ".hpp",
  ".hxx",
  ".sh",
  ".bash",
  ".zsh",
  ".fish",
];

const FORBIDDEN_MARKERS = [
  "python",
  "cocotb",
  ".py",
  ".pyc",
  ".pyo",
  "requirements.txt",
  "pyproject.toml",
  "setup.py",
  "setup.cfg",
  "tox.ini",
  ".python-version",
  "poetry.lock",
  "pipfile",
  "pipfile.lock",
  "package.json",
  "requirements-dev.txt",
];

const SCAN_EXT = [
  ".rs",
  ".ts",
  ".sv",
  ".v",
  ".svh",
  ".tcl",
  ".xdc",
  ".pcf",
  ".lpf",
  ".qsf",
  ".pdc",
  ".cst",
  ".sdc",
  ".mk",
  ".yml",
  ".yaml",
  ".toml",
];

const SKIP_DIR_MARKERS = [
  "/.git/",
  "/.codex/",
  "/.af-build/",
  "/artifacts/",
  "/target/",
  "/obj_dir/",
];

const POLICY_CONTENT_ALLOWLIST = [
  "/tools/ts/no_python_check.ts",
  "/tools/ts/repo_audit.ts",
  "/tools/ts/mod.ts",
  "/tools/ts/manifest_check.ts",
  "/tools/ts/license_check.ts",
  "/crates/af-backend-litex/src/lib.rs",
  "/crates/af-backend-litex/tests/litex_skeleton.rs",
  "/crates/af-cli/src/main.rs",
  "/crates/af-cli/src/tooling.rs",
  "/crates/af-cli/tests/build_clean.rs",
  "/crates/af-cli/tests/cli.rs",
  "/crates/af-cli/tests/wrapper_subcommands.rs",
  "/crates/af-manifest/src/lib.rs",
  "/crates/af-security/src/lib.rs",
  "/crates/af-wrapper-gen/tests/wrapper_targets.rs",
  "/crates/xtask/src/main.rs",
  "/crates/xtask/src/tasks_new.rs",
  "/af-toolchain.toml",
  "/examples/af-mod-add/af-core.toml",
];

const POLICY_PATH_ALLOWLIST = [
  "/.claude/skills/af-cli-contract-guard/check.sh",
  "/.claude/skills/af-error-explainer/test.sh",
  "/scripts/check-af-skills.sh",
  "/scripts/docker-smoke.sh",
  "/scripts/install-af-codex-skills.sh",
  "/scripts/install-core-integration-tools.sh",
  "/scripts/install-oss-hdl-tools.sh",
  "/scripts/install-smt-solvers.sh",
  "/scripts/oss-hdl-smoke.sh",
  "/scripts/pre-install.sh",
];

function ext(path: string): string {
  const last = path.lastIndexOf(".");
  return last >= 0 ? path.slice(last).toLowerCase() : "";
}

function normalizePath(path: string): string {
  return path.replaceAll("\\", "/").toLowerCase();
}

function shouldSkipDir(path: string): boolean {
  const normalized = `/${normalizePath(path)}/`;
  if (SKIP_DIR_MARKERS.some((marker) => normalized.includes(marker))) {
    return true;
  }
  return false;
}

function isPolicyContentFile(path: string): boolean {
  const normalized = `/${normalizePath(path)}`;
  return POLICY_CONTENT_ALLOWLIST.some((suffix) => normalized.endsWith(suffix));
}

function isPolicyPathFile(path: string): boolean {
  const normalized = `/${normalizePath(path)}`;
  return POLICY_PATH_ALLOWLIST.some((suffix) => normalized.endsWith(suffix));
}

function shouldScanContent(path: string): boolean {
  return SCAN_EXT.includes(ext(path));
}

export async function checkNoPython(root = Deno.cwd()): Promise<boolean> {
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop()!;
    for await (const entry of Deno.readDir(current)) {
      if (
        entry.name === ".git" || entry.name === ".codex" ||
        entry.name === ".af-build" || entry.name === "artifacts" ||
        entry.name === "target" ||
        entry.name === "obj_dir"
      ) {
        continue;
      }

      const path = `${current}/${entry.name}`;
      if (FORBIDDEN_NAME_SET.has(entry.name.toLowerCase())) {
        throw new Error(`forbidden file name/pattern: ${path}`);
      }

      if (entry.isDirectory) {
        if (!shouldSkipDir(path)) {
          stack.push(path);
        }
        continue;
      }

      if (!entry.isFile) {
        continue;
      }

      if (isPolicyPathFile(path)) {
        continue;
      }

      const extension = ext(entry.name);
      if (FORBIDDEN_EXTENSIONS.includes(extension)) {
        throw new Error(`forbidden file extension: ${path}`);
      }

      if (!shouldScanContent(path) || isPolicyContentFile(path)) {
        continue;
      }

      const content = await Deno.readTextFile(path);
      const loweredContent = content.toLowerCase();
      for (const marker of FORBIDDEN_MARKERS) {
        if (loweredContent.includes(marker)) {
          throw new Error(`forbidden marker in file content: ${path}`);
        }
      }
    }
  }
  return true;
}
