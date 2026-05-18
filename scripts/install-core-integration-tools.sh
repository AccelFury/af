#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

usage() {
  cat <<'USAGE'
Install core integration tools used by af wrapper/package flows.

Installs/checks:
  - xmllint (provided by libxml2-utils on Debian/Ubuntu)
  - FuseSoC
  - Edalize Python module

Supported host package manager:
  - apt on Debian/Ubuntu for xmllint and Python venv prerequisites

Python packages are installed into an af-managed virtualenv by default. This
keeps host Python packages isolated while still letting af checks see them when
the virtualenv bin directory is added to PATH.

Usage:
  scripts/install-core-integration-tools.sh [options]

Options:
  --yes                   Do not prompt before package installation.
  --dry-run               Print actions without changing the host.
  --venv <path>           Python virtualenv path.
                          Default: .af-tools/python
  --help                  Show this help.

Examples:
  scripts/install-core-integration-tools.sh --yes
  scripts/install-core-integration-tools.sh --dry-run
  PATH="$PWD/.af-tools/python/bin:$PATH" cargo run -p af-cli --bin af -- core tooling my-core --json

Notes:
  - This script does not install vendor EDA tools.
  - Use scripts/install-oss-hdl-tools.sh for simulator/synthesis tools.
  - Use scripts/install-smt-solvers.sh for formal SMT solvers.
USAGE
}

YES=0
DRY_RUN=0
VENV=".af-tools/python"

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
    --venv)
      if (($# < 2)); then
        echo "error: --venv requires a value" >&2
        exit 2
      fi
      VENV="$2"
      shift 2
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

run() {
  printf '+'
  printf ' %q' "$@"
  printf '\n'
  if ((DRY_RUN == 0)); then
    "$@"
  fi
}

need_command() {
  local program="$1"
  if ! command -v "${program}" >/dev/null 2>&1; then
    echo "error: required command '${program}' is not available" >&2
    exit 3
  fi
}

sudo_prefix=()
if ((EUID != 0)); then
  need_command sudo
  sudo_prefix=(sudo)
fi

if [[ ! -r /etc/os-release ]]; then
  echo "error: cannot detect host OS; /etc/os-release is missing" >&2
  exit 4
fi

# shellcheck disable=SC1091
. /etc/os-release
case "${ID:-}:${ID_LIKE:-}" in
  debian:*|ubuntu:*|*:debian*|*:ubuntu*)
    ;;
  *)
    echo "error: unsupported OS '${PRETTY_NAME:-unknown}'; this script currently supports apt-based Debian/Ubuntu hosts" >&2
    exit 4
    ;;
esac

need_command apt-get

apt_packages=(ca-certificates libxml2-utils python3 python3-pip python3-venv)
python_packages=(edalize fusesoc)

if ((YES == 0 && DRY_RUN == 0)); then
  echo "This will install apt packages: ${apt_packages[*]}"
  echo "It will install Python packages into ${VENV}: ${python_packages[*]}"
  read -r -p "Continue? [y/N] " answer
  case "${answer}" in
    y|Y|yes|YES)
      ;;
    *)
      echo "aborted"
      exit 1
      ;;
  esac
fi

run "${sudo_prefix[@]}" apt-get update
run "${sudo_prefix[@]}" apt-get install -y --no-install-recommends "${apt_packages[@]}"
run python3 -m venv "${VENV}"
run "${VENV}/bin/python" -m pip install --upgrade pip
run "${VENV}/bin/python" -m pip install --upgrade "${python_packages[@]}"

if ((DRY_RUN == 1)); then
  echo "dry-run complete; no packages were installed and no version checks were executed."
  exit 0
fi

check_version() {
  local tool="$1"
  shift
  if command -v "${tool}" >/dev/null 2>&1; then
    echo "ok: ${tool}"
    "${tool}" "$@" 2>&1 | sed -n '1,3p'
  else
    echo "missing: ${tool}" >&2
    return 1
  fi
}

check_version xmllint --version
check_version "${VENV}/bin/fusesoc" --version
"${VENV}/bin/python" -c 'import edalize; print("edalize:", getattr(edalize, "__version__", "import ok"))'

cat <<EOF
Core integration tool installation check completed.

To let af core/tooling probes use this virtualenv in the current shell, run:

  export PATH="$(pwd)/${VENV}/bin:\$PATH"
EOF
