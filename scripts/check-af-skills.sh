#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Read-only freshness guard for AccelFury `af-*` skills, agents, and rules.

set -euo pipefail

repo="$(git rev-parse --show-toplevel)"
cd "$repo"

AF_BIN="${AF_BIN:-target/debug/af}"
codex_home="${CODEX_HOME:-$HOME/.codex}"
if [[ ! -x "$AF_BIN" ]]; then
  echo "FATAL: AF_BIN is not executable: $AF_BIN" >&2
  echo "Build af first or pass AF_BIN=/path/to/af." >&2
  exit 3
fi

checked_files=(
  CLAUDE.md
  AGENTS.md
  .codex/AGENTS.md
  skills/README.md
  skills/af-*/SKILL.md
  skills/af-*/agents/*
  skills/af-*/references/*
  .claude/skills/af-*/SKILL.md
  .claude/agents/af-*.md
  scripts/install-af-codex-skills.sh
)
shopt -s nullglob
project_codex_skills=(skills/af-*)
if [[ ${#project_codex_skills[@]} -eq 0 ]]; then
  echo "FATAL: no project Codex skills found under skills/af-*" >&2
  exit 3
fi
if compgen -G "$codex_home/skills/af-*/SKILL.md" >/dev/null; then
  for f in "$codex_home"/skills/af-*/* "$codex_home"/skills/af-*/references/*; do
    if [[ -f "$f" ]]; then
      checked_files+=("$f")
    fi
  done
fi

run() {
  echo "== $* =="
  "$@"
}

check_absent_fixed() {
  local needle="$1"
  local label="$2"
  if rg -n -F -- "$needle" "${checked_files[@]}"; then
    echo "stale skill text: $label" >&2
    stale=1
  fi
}

run bash .claude/skills/af-error-explainer/test.sh
run bash .claude/skills/af-cli-contract-guard/check.sh
run "$AF_BIN" manifest migrate --help
run "$AF_BIN" evidence ingest --help
run "$AF_BIN" wrapper generate --help
run "$AF_BIN" ci init --help
run "$AF_BIN" core verify --help

stale=0
check_absent_fixed "af evidence ingest --kind board-bringup --input" "invalid evidence kind board-bringup"
check_absent_fixed "manifest migrate <core_dir>/af-core.toml --write --json" "old manifest migrate syntax"
check_absent_fixed '"status": "blocked|planned|absent"' "old maturity status set without not-applicable"
check_absent_fixed 'Update `[sources].files` to include both the generic file and the wrapper' "vendor wrapper in portable sources"
check_absent_fixed "lint/sim/synth/wrapper/optional CI-ingest" "old evidence-refresh summary with synth wording"
check_absent_fixed "runtime skill bundles" "old Codex runtime bundle wording"
check_absent_fixed "docs/agent/**" "stale docs/agent directory surface"
check_absent_fixed "docs/agent/todo-issues.jsonl" "stale docs/agent todo ledger"
check_absent_fixed "docs/agent/agent-operating-protocol.yaml" "stale docs/agent operating protocol"
check_absent_fixed "runtime/registries/af-freshness.json" "mandatory runtime freshness registry"
check_absent_fixed "runtime/registries/af-surface-registry.json" "mandatory runtime surface registry"

for dir in "${project_codex_skills[@]}"; do
  name="$(basename "$dir")"
  installed="$codex_home/skills/$name"
  if [[ -d "$installed" ]] && ! diff -qr "$dir" "$installed" >/tmp/af-skill-mirror.diff 2>&1; then
    echo "stale skill mirror: $installed differs from project golden source $dir" >&2
    cat /tmp/af-skill-mirror.diff >&2
    stale=1
  fi
done
rm -f /tmp/af-skill-mirror.diff

if rg -n -F -- ".codex/work/public-prep" CLAUDE.md AGENTS.md skills .claude/skills .claude/agents "$codex_home"/skills/af-* 2>/dev/null; then
  echo "stale skill text: active surfaces must not point at quarantined .codex/work/public-prep" >&2
  stale=1
fi

if (( stale )); then
  exit 1
fi

echo "af skills check passed"
