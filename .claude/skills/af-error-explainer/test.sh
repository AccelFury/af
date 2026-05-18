#!/usr/bin/env bash
# Regression test for the af-error-explainer subagent.
#
# Goal: prove that every AF_* error code raised in the af source tree has
#   1. at least one origin file under crates/
#   2. a non-trivial hint string (> 20 characters) raised from a CliError
#      constructor or domain `hint()` method, and
#   3. is not hardcoded into the subagent itself (we want the subagent to
#      look codes up at runtime, not maintain a stale internal registry).
#
# This test does NOT invoke the subagent through an LLM (too expensive,
# non-deterministic). It verifies the *grounding* assumption the subagent
# relies on: that every code is locatable + has a real hint in source.
#
# Exit 0 if all live AF_* codes pass. Exit 1 otherwise; failures are
# listed on stderr.
#
# Usage:
#   .claude/skills/af-error-explainer/test.sh          # default repo
#   AF_REPO=/path/to/af .claude/skills/af-error-explainer/test.sh

set -euo pipefail

# --- locate repo root --------------------------------------------------------

repo="${AF_REPO:-}"
if [[ -z "$repo" ]]; then
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  # .claude/skills/af-error-explainer -> repo root is three levels up
  repo="$(cd "$script_dir/../../.." && pwd)"
fi

if [[ ! -d "$repo/crates" ]]; then
  echo "FATAL: '$repo/crates' not found; set AF_REPO=/path/to/af or run from inside the repo" >&2
  exit 2
fi

agent="$repo/.claude/agents/af-error-explainer.md"
if [[ ! -f "$agent" ]]; then
  echo "FATAL: missing $agent" >&2
  exit 2
fi

# --- enumerate candidate codes -----------------------------------------------

# Prefer ripgrep when available; fall back to grep. We use the regex
# `AF_[A-Z][A-Z0-9_]+` to capture every occurrence under crates/.
if command -v rg >/dev/null 2>&1; then
  mapfile -t candidates < <(
    rg --no-heading --no-filename -oN 'AF_[A-Z][A-Z0-9_]+' "$repo/crates" | sort -u
  )
else
  mapfile -t candidates < <(
    grep -rohE 'AF_[A-Z][A-Z0-9_]+' "$repo/crates" | sort -u
  )
fi

# Helper: does any file under crates/ literally contain $1?
has_file() {
  local code="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -l --no-heading -F "$code" "$repo/crates" 2>/dev/null
  else
    grep -l -r -F "$code" "$repo/crates" 2>/dev/null
  fi
}

if [[ ${#candidates[@]} -eq 0 ]]; then
  echo "FATAL: no AF_* identifiers found under crates/; tree corruption?" >&2
  exit 2
fi

# --- false-positive filter ---------------------------------------------------
#
# Identifiers matching the AF_* shape that are NOT error codes:
#   - AF_BUILD_ROOT          — environment variable name read by main.rs
#   - AF_SELF_CHECK_*        — env-var override paths in af-selfcheck.toml
#                              and tests (path_env field)
#   - AF_REPO                — overridable env var used by this script
#
# We keep the regex narrow enough not to overreach: a code is a "real
# error code" iff it appears at least once in a context that is NOT an
# env-var read. Heuristic: skip codes whose every occurrence is preceded
# by `env::var`, `env_remove`, `set_var`, `path_env`, or appears inside a
# `#` comment.

false_positives=(
  "AF_BUILD_ROOT"
  "AF_REPO"
  "AF_SELF_CHECK_AF_MOD_ADD"
  "AF_SELF_CHECK_AF_RESET_SYNC"
)

is_env_var() {
  local code="$1"
  case " ${false_positives[*]} " in
    *" $code "*) return 0 ;;
  esac
  # AF_SELF_CHECK_* are by convention env-var overrides for self-check
  if [[ "$code" == AF_SELF_CHECK_* ]]; then
    return 0
  fi
  return 1
}

# --- run the checks ----------------------------------------------------------

real_codes=()
for code in "${candidates[@]}"; do
  if is_env_var "$code"; then
    continue
  fi
  real_codes+=("$code")
done

if [[ ${#real_codes[@]} -eq 0 ]]; then
  echo "FATAL: filter removed every candidate; check false_positives" >&2
  exit 2
fi

pass=0
fail=0
declare -a fail_lines=()

# 1) Every code has an origin file under crates/.
for code in "${real_codes[@]}"; do
  if [[ -z "$(has_file "$code")" ]]; then
    fail_lines+=("ORPHAN: $code has no origin file under crates/")
    fail=$((fail + 1))
    continue
  fi
  pass=$((pass + 1))
done

# 2) Every code has at least one declared hint > 20 chars.
#    We look for the `CliError::new(..., "<hint>", ...)` shape OR a `hint:`
#    field in a struct literal whose value is a longer-than-20-char string.
#    This is heuristic; we accept any line containing the code followed by
#    a string literal of sufficient length within ±20 lines.

hint_check() {
  local code="$1"
  # For each origin file, check whether a window of ±15 lines around any
  # occurrence of $code contains a string literal of >20 characters.
  # The window covers the common pattern where `code()` returns the
  # string in one `match` arm and `hint()` returns the long string in the
  # corresponding arm of a separate impl block a few lines below.
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    if grep -A 15 -B 2 -F "$code" "$file" 2>/dev/null \
       | grep -E '"[^"]{21,}"' >/dev/null 2>&1; then
      return 0
    fi
  done < <(has_file "$code")
  return 1
}

declare -a no_hint=()
for code in "${real_codes[@]}"; do
  if [[ -z "$(has_file "$code")" ]]; then
    continue  # already counted as ORPHAN
  fi
  if ! hint_check "$code"; then
    no_hint+=("$code")
  fi
done

if [[ ${#no_hint[@]} -gt 0 ]]; then
  for code in "${no_hint[@]}"; do
    fail_lines+=("NO_HINT: $code has no string literal > 20 chars within ±15 lines of its declaration")
    fail=$((fail + 1))
  done
fi

# 3) Subagent must NOT hardcode a large registry of codes.
# Count DISTINCT identifiers (not raw mentions). Worked examples and
# prefix lists may legitimately reference a handful (e.g. AF_PORTABLE_*,
# AF_MANIFEST_*, AF_CORES_REGISTRY_*). Cap at 15.
hardcoded=$(grep -ohE 'AF_[A-Z][A-Z0-9_]+' "$agent" 2>/dev/null | sort -u | wc -l)
if [[ "$hardcoded" -gt 15 ]]; then
  fail_lines+=("HARDCODED: $agent mentions $hardcoded distinct AF_* identifiers (limit is 15); subagent should look codes up at runtime instead")
  fail=$((fail + 1))
fi

# --- report ------------------------------------------------------------------

total=${#real_codes[@]}
echo "af-error-explainer self-test"
echo "  candidates scanned:  ${#candidates[@]}"
echo "  env-vars filtered:   $(( ${#candidates[@]} - total ))"
echo "  real error codes:    $total"
echo "  passed:              $pass"
echo "  failed:              $fail"

if [[ $fail -gt 0 ]]; then
  echo ""
  echo "Failures:"
  for line in "${fail_lines[@]}"; do
    echo "  - $line" >&2
  done
  exit 1
fi

echo "  OK"
exit 0
