#!/usr/bin/env bash
# Executable contract guard for `af`. Mirrors the checklist in SKILL.md.
#
# Exit codes:
#   0 — SAFE or ADDITIVE-only, smoke checks green.
#   1 — BREAKING change without a companion bump.
#   2 — smoke regression (registry/self-check/manifest validate failed).
#   3 — internal error (git not available, repo invalid).
#
# Usage:
#   .claude/skills/af-cli-contract-guard/check.sh           # diff against HEAD
#   AF_GUARD_BASE=main .claude/skills/af-cli-contract-guard/check.sh

set -u

base="${AF_GUARD_BASE:-HEAD}"
repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo" ]]; then
  echo "FATAL: not inside a git repository" >&2
  exit 3
fi
cd "$repo"

surface_paths=(
  crates/af-cli/src/main.rs
  crates/af-cli/src/commands/
  crates/af-cli/src/cores_registry.rs
  crates/af-manifest/src/lib.rs
  crates/af-complexity/src/lib.rs
  crates/af-report/src/lib.rs
  schemas/
  registries/cores.registry.json
  docs/cli-reference.md
  docs/licensing.md
  docs/manifest-reference.md
)

# --- Step 1: changed surface files -------------------------------------------
mapfile -t changed < <(
  git diff --name-only "$base" -- "${surface_paths[@]}" 2>/dev/null
)
if [[ ${#changed[@]} -eq 0 ]]; then
  echo "## Contract guard summary"
  echo ""
  echo "Changed contract files: 0"
  echo "Nothing to guard. Smoke checks still run below."
  smoke_only=1
else
  smoke_only=0
fi

# --- Step 2: per-surface diagnostics -----------------------------------------
findings=()
add_finding() { findings+=("$1"); }

if [[ $smoke_only -eq 0 ]]; then
  # CLI surface: removed clap variants / arg(long) lines
  if git diff "$base" -- crates/af-cli/src/main.rs crates/af-cli/src/commands/ 2>/dev/null \
     | grep -E '^-[^-].*(#\[command|#\[arg|Subcommand|enum [A-Z][a-zA-Z]*Command|"AF_)' \
     >/tmp/af-guard-cli-removed.txt 2>&1; then
    if [[ -s /tmp/af-guard-cli-removed.txt ]]; then
      add_finding "[REVIEW NEEDED] CLI surface removals/renames detected (see crates/af-cli/src/main.rs diff). Confirm whether SemVer bump or CHANGELOG entry is required."
    fi
  fi
  rm -f /tmp/af-guard-cli-removed.txt

  # JSON shapes: removed pub field/struct/enum lines in af-report or af-complexity
  removed_pub=$(git diff "$base" -- crates/af-report/src/lib.rs crates/af-complexity/src/lib.rs 2>/dev/null \
    | grep -cE '^-[[:space:]]*pub (struct|enum|fn) |^-[[:space:]]*pub [a-z_]+:' || true)
  if [[ "$removed_pub" -gt 0 ]]; then
    add_finding "[BREAKING] $removed_pub removed public-field/struct/enum line(s) in af-report or af-complexity. Bump AfReport report_version / schema_version and add CHANGELOG entry."
  fi

  # Error codes: removed
  if [[ -d crates ]]; then
    {
      for f in $(git ls-tree -r "$base" --name-only 2>/dev/null | grep -E '^crates/.*/src/.*\.rs$'); do
        git show "$base:$f" 2>/dev/null
      done
    } | grep -ohE 'AF_[A-Z][A-Z0-9_]+' | sort -u > /tmp/af-guard-codes-base.txt 2>/dev/null
    grep -rohE 'AF_[A-Z][A-Z0-9_]+' crates/ 2>/dev/null | sort -u > /tmp/af-guard-codes-now.txt
    removed=$(comm -23 /tmp/af-guard-codes-base.txt /tmp/af-guard-codes-now.txt 2>/dev/null | grep -v '^$' || true)
    added=$(comm -13 /tmp/af-guard-codes-base.txt /tmp/af-guard-codes-now.txt 2>/dev/null | grep -v '^$' || true)
    if [[ -n "$removed" ]]; then
      add_finding "[BREAKING] error code(s) removed: $(echo "$removed" | tr '\n' ' '). Add CHANGELOG entry under Unreleased."
    fi
    if [[ -n "$added" ]]; then
      add_finding "[ADDED] error code(s) introduced: $(echo "$added" | tr '\n' ' '). Recommend CHANGELOG line."
    fi
    rm -f /tmp/af-guard-codes-base.txt /tmp/af-guard-codes-now.txt
  fi

  # Manifest schema: removed pub fields in CoreManifest or schema property
  removed_manifest=$(git diff "$base" -- crates/af-manifest/src/lib.rs schemas/af-core.schema.json 2>/dev/null \
    | grep -cE '^-[[:space:]]*pub [a-z_]+:|^-[[:space:]]*"[a-z_]+":[[:space:]]*\{' || true)
  if [[ "$removed_manifest" -gt 0 ]]; then
    add_finding "[REVIEW NEEDED] $removed_manifest possibly-removed manifest field(s) or schema property. If removed, bump af_version and update docs/manifest-reference.md."
  fi

  # Registry: removed core_id entries
  removed_cores=$(git diff "$base" -- registries/cores.registry.json 2>/dev/null \
    | grep -cE '^-[[:space:]]*"core_id":' || true)
  if [[ "$removed_cores" -gt 0 ]]; then
    add_finding "[BREAKING] $removed_cores removed core_id entry/entries in registries/cores.registry.json. Consumers (skills, fpga.chat) may depend on them."
  fi

  # Tier mapping changes
  tier_diff=$(git diff "$base" -- crates/af-cli/src/main.rs 2>/dev/null | sed -n '/fn tier_required_rows/,/^}/p')
  if [[ -n "$tier_diff" ]]; then
    add_finding "[REVIEW NEEDED] tier_required_rows changed. Verify whether existing cores still satisfy verified-package / enterprise."
  fi

  # Schema autogen drift
  if git diff --name-only "$base" -- crates/af-report/src/lib.rs 2>/dev/null | grep -q lib.rs; then
    if ! git diff --name-only "$base" -- schemas/af-report.schema.json 2>/dev/null | grep -q af-report.schema.json; then
      add_finding "[ADDED] crates/af-report/src/lib.rs changed but schemas/af-report.schema.json did not. Regenerate with: cargo run --quiet --example dump_schema -p af-report > schemas/af-report.schema.json"
    fi
  fi
fi

# --- Step 3: smoke checks (always) -------------------------------------------
smoke_fail=0
run_smoke() {
  local label="$1"; shift
  if ! "$@" >/tmp/af-guard-smoke.log 2>&1; then
    smoke_fail=1
    echo "[SMOKE REGRESSION] $label failed:" >&2
    tail -20 /tmp/af-guard-smoke.log >&2
  fi
}

run_smoke "af registry check --json"          cargo run --quiet -p af-cli --bin af -- registry check --json
run_smoke "af self check --json"              cargo run --quiet -p af-cli --bin af -- self check --json
run_smoke "af manifest validate af-reset-sync" cargo run --quiet -p af-cli --bin af -- manifest validate examples/af-reset-sync/af-core.toml --json
rm -f /tmp/af-guard-smoke.log

# --- Step 4: verdict ---------------------------------------------------------
breaking=0
added=0
for f in "${findings[@]}"; do
  case "$f" in
    \[BREAKING\]*) breaking=$((breaking + 1)) ;;
    \[ADDED\]*)    added=$((added + 1)) ;;
  esac
done

echo ""
echo "## Contract guard summary"
echo ""
echo "Changed contract files: ${#changed[@]}"
echo "Breaking findings: $breaking"
echo "Additive findings: $added"
echo "Review-needed findings: $(( ${#findings[@]} - breaking - added ))"

if [[ ${#findings[@]} -gt 0 ]]; then
  echo ""
  echo "## Findings"
  for f in "${findings[@]}"; do
    echo "- $f"
  done
fi

echo ""
echo "## Verdict"
if [[ $smoke_fail -ne 0 ]]; then
  echo "❌ SMOKE REGRESSION — at least one smoke check failed (independent of diff)."
  exit 2
elif [[ $breaking -gt 0 ]]; then
  echo "❌ BREAKING WITHOUT COMPANION — review findings; CHANGELOG / version bump required."
  exit 1
elif [[ $added -gt 0 ]]; then
  echo "⚠️  ADDITIVE ONLY — smoke green; consider adding a CHANGELOG line under Unreleased."
  exit 0
else
  echo "✅ SAFE — no contract surfaces changed (or only review-needed advisories); smoke green."
  exit 0
fi
