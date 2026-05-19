// SPDX-License-Identifier: AGPL-3.0-or-later
const POLICY_MARKERS = [
  "## Test Design Obligation",
  "thoughtful tests",
  "closest existing coverage",
];

const ROOT_POLICY_FILES = [
  "CLAUDE.md",
  "docs/agent-workflow.md",
];

async function fileExists(path: string): Promise<boolean> {
  const stat = await Deno.stat(path).catch(() => null);
  return stat?.isFile === true;
}

async function collectAgentPolicyFiles(root: string): Promise<string[]> {
  const files = [...ROOT_POLICY_FILES];

  for await (const entry of Deno.readDir(`${root}/.claude/agents`)) {
    if (entry.isFile && entry.name.endsWith(".md")) {
      files.push(`.claude/agents/${entry.name}`);
    }
  }

  for await (const entry of Deno.readDir(`${root}/.claude/skills`)) {
    if (!entry.isDirectory) {
      continue;
    }
    const rel = `.claude/skills/${entry.name}/SKILL.md`;
    if (await fileExists(`${root}/${rel}`)) {
      files.push(rel);
    }
  }

  files.sort();
  return files;
}

export async function checkAgentPolicy(root = Deno.cwd()): Promise<boolean> {
  const files = await collectAgentPolicyFiles(root);
  const failures: string[] = [];

  for (const rel of files) {
    const text = await Deno.readTextFile(`${root}/${rel}`);
    const normalized = text.replace(/\s+/g, " ");
    const missing = POLICY_MARKERS.filter((marker) =>
      !normalized.includes(marker)
    );
    if (missing.length > 0) {
      failures.push(`${rel}: missing ${missing.join(", ")}`);
    }
  }

  const testingStrategy = await Deno.readTextFile(
    `${root}/docs/testing-strategy.md`,
  );
  if (
    !testingStrategy.includes("## Agent obligation") ||
    !testingStrategy.includes("AI/LLM agents")
  ) {
    failures.push(
      "docs/testing-strategy.md: missing agent test-design obligation",
    );
  }

  if (failures.length > 0) {
    throw new Error(`agent test-policy check failed:\n${failures.join("\n")}`);
  }

  return true;
}

if (import.meta.main) {
  await checkAgentPolicy();
  console.log("agent test-policy check passed");
}
