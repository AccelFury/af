#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

usage() {
  cat <<'USAGE'
Install the host-side open-source tools used by af development workflows.

This is a convenience orchestrator over the focused installers:

  - scripts/install-oss-hdl-tools.sh
  - scripts/install-smt-solvers.sh
  - scripts/install-core-integration-tools.sh

It installs/checks:

  - Icarus Verilog: iverilog, vvp
  - Verilator
  - Yosys
  - optional nextpnr packages when the distribution provides them
  - sby / SymbiYosys, from apt or upstream source
  - SMT solvers: boolector, z3, yices-smt2, bitwuzla, cvc5
  - core integration tools: xmllint, FuseSoC, Edalize

Supported host package manager:

  - apt on Debian/Ubuntu

Usage:

  scripts/pre-install.sh [options]

Options:

  --yes                     Do not prompt before installation.
  --dry-run                 Print actions without changing the host.
  --allow-missing-smt       Do not fail when a solver cannot be installed.
  --skip-hdl                Skip simulator/synthesis/PnR tool installer.
  --skip-smt                Skip SMT solver installer.
  --skip-integration        Skip xmllint/FuseSoC/Edalize installer.
  --skip-sby                Install/check HDL tools without requiring sby.
  --no-sby-source           Do not install sby from upstream source.
  --no-yices-binary         Do not install Yices from the official tarball.
  --no-bitwuzla-source      Do not build Bitwuzla from upstream source.
  --venv <path>             Python virtualenv for FuseSoC/Edalize.
                            Default: .af-tools/python
  --prefix <path>           Install prefix for source/binary installs.
                            Default: /usr/local
  --sby-ref <ref>           Optional git ref for sby source install.
  --bitwuzla-ref <ref>      Git tag/branch for Bitwuzla source install.
                            Default is owned by install-smt-solvers.sh.
  --skip-check              Do not run final af tooling check.
  --help                    Show this help.

Examples:

  scripts/pre-install.sh --yes
  scripts/pre-install.sh --dry-run
  scripts/pre-install.sh --yes --allow-missing-smt

After installation, expose the af-managed Python tools in your shell:

  export PATH="$PWD/.af-tools/python/bin:$PATH"

Notes:

  - This script does not install Rust/Cargo.
  - This script does not install vendor EDA tools.
  - Networked upstream installs are explicit through the source/binary defaults
    above and can be disabled with the --no-* flags.
USAGE
}

YES=0
DRY_RUN=0
ALLOW_MISSING_SMT=0
SKIP_HDL=0
SKIP_SMT=0
SKIP_INTEGRATION=0
SKIP_SBY=0
WITH_SBY_SOURCE=1
WITH_YICES_BINARY=1
WITH_BITWUZLA_SOURCE=1
SKIP_CHECK=0
VENV=".af-tools/python"
PREFIX="/usr/local"
SBY_REF=""
BITWUZLA_REF=""

while (($# > 0)); do
  case "$1" in
    --yes)
      YES=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --allow-missing-smt)
      ALLOW_MISSING_SMT=1
      shift
      ;;
    --skip-hdl)
      SKIP_HDL=1
      shift
      ;;
    --skip-smt)
      SKIP_SMT=1
      shift
      ;;
    --skip-integration)
      SKIP_INTEGRATION=1
      shift
      ;;
    --skip-sby)
      SKIP_SBY=1
      shift
      ;;
    --no-sby-source)
      WITH_SBY_SOURCE=0
      shift
      ;;
    --no-yices-binary)
      WITH_YICES_BINARY=0
      shift
      ;;
    --no-bitwuzla-source)
      WITH_BITWUZLA_SOURCE=0
      shift
      ;;
    --venv)
      if (($# < 2)); then
        echo "error: --venv requires a value" >&2
        exit 2
      fi
      VENV="$2"
      shift 2
      ;;
    --prefix)
      if (($# < 2)); then
        echo "error: --prefix requires a value" >&2
        exit 2
      fi
      PREFIX="$2"
      shift 2
      ;;
    --sby-ref)
      if (($# < 2)); then
        echo "error: --sby-ref requires a value" >&2
        exit 2
      fi
      SBY_REF="$2"
      shift 2
      ;;
    --bitwuzla-ref)
      if (($# < 2)); then
        echo "error: --bitwuzla-ref requires a value" >&2
        exit 2
      fi
      BITWUZLA_REF="$2"
      shift 2
      ;;
    --skip-check)
      SKIP_CHECK=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option '$1'" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "${VENV}" in
  ""|"/"|".")
    echo "error: refusing unsafe virtualenv path '${VENV}'" >&2
    exit 2
    ;;
esac

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
cd "${repo_root}"

run_script() {
  printf '== %s ==\n' "$*"
  "$@"
}

common_args=()
if ((YES == 1)); then
  common_args+=(--yes)
fi
if ((DRY_RUN == 1)); then
  common_args+=(--dry-run)
fi

if ((SKIP_HDL == 0)); then
  hdl_args=("${common_args[@]}" --prefix "${PREFIX}")
  if ((SKIP_SBY == 1)); then
    hdl_args+=(--skip-sby)
  elif ((WITH_SBY_SOURCE == 1)); then
    hdl_args+=(--with-sby-source)
  fi
  if [[ -n "${SBY_REF}" ]]; then
    hdl_args+=(--sby-ref "${SBY_REF}")
  fi
  run_script scripts/install-oss-hdl-tools.sh "${hdl_args[@]}"
else
  echo "skipped: HDL/simulation tool installer"
fi

if ((SKIP_SMT == 0)); then
  smt_args=("${common_args[@]}" --prefix "${PREFIX}")
  if ((ALLOW_MISSING_SMT == 1)); then
    smt_args+=(--allow-missing)
  fi
  if ((WITH_YICES_BINARY == 1)); then
    smt_args+=(--with-yices-binary)
  fi
  if ((WITH_BITWUZLA_SOURCE == 1)); then
    smt_args+=(--with-bitwuzla-source)
  fi
  if [[ -n "${BITWUZLA_REF}" ]]; then
    smt_args+=(--bitwuzla-ref "${BITWUZLA_REF}")
  fi
  run_script scripts/install-smt-solvers.sh "${smt_args[@]}"
else
  echo "skipped: SMT solver installer"
fi

if ((SKIP_INTEGRATION == 0)); then
  integration_args=("${common_args[@]}" --venv "${VENV}")
  run_script scripts/install-core-integration-tools.sh "${integration_args[@]}"
else
  echo "skipped: core integration tool installer"
fi

venv_abs="${VENV}"
case "${venv_abs}" in
  /*)
    ;;
  *)
    venv_abs="${repo_root}/${VENV}"
    ;;
esac

cat <<EOF
Pre-install completed.

To expose af-managed Python tools in this shell, run:

  export PATH="${venv_abs}/bin:\$PATH"
EOF

if ((DRY_RUN == 1 || SKIP_CHECK == 1)); then
  exit 0
fi

check_tools=()
if ((SKIP_HDL == 0)); then
  check_tools+=(
    iverilog
    vvp
    yosys
    nextpnr-ice40
    nextpnr-ecp5
    nextpnr-gowin
    sby
    verilator
  )
fi
if ((SKIP_SMT == 0)); then
  check_tools+=(
    boolector
    z3
    yices-smt2
    bitwuzla
    cvc5
  )
fi
if ((SKIP_INTEGRATION == 0)); then
  check_tools+=(
    xmllint
    fusesoc
    edalize
  )
fi

if ((${#check_tools[@]} == 0)); then
  echo "skipped: final af tooling visibility check has no selected tool groups"
elif command -v cargo >/dev/null 2>&1; then
  IFS=,
  tool_csv="${check_tools[*]}"
  unset IFS
  echo "== af tooling visibility check =="
  PATH="${venv_abs}/bin:${PATH}" cargo run -p af-cli --bin af -- tooling check --tools "${tool_csv}" --json
else
  echo "warning: cargo is not available; install Rust/Cargo before building or running af." >&2
fi
