#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

usage() {
  cat <<'USAGE'
Install open-source HDL tools used by af.

Installs/checks:
  - iverilog
  - vvp (provided by the iverilog package on Debian/Ubuntu)
  - yosys
  - nextpnr-ice40
  - nextpnr-ecp5
  - nextpnr-gowin
  - verilator
  - sby / SymbiYosys

Supported host package manager:
  - apt on Debian/Ubuntu

Usage:
  scripts/install-oss-hdl-tools.sh [options]

Options:
  --yes                   Do not prompt before package installation.
  --dry-run               Print actions without changing the host.
  --skip-sby              Do not require sby after installing other tools.
  --with-sby-source       If no apt package provides sby, install it from
                          https://github.com/YosysHQ/sby.git.
  --sby-ref <ref>         Optional git ref for --with-sby-source.
  --prefix <path>         Install prefix for --with-sby-source.
                          Default: /usr/local
  --help                  Show this help.

Examples:
  scripts/install-oss-hdl-tools.sh --yes
  scripts/install-oss-hdl-tools.sh --yes --with-sby-source
  scripts/install-oss-hdl-tools.sh --dry-run --with-sby-source

Notes:
  - This script does not install vendor EDA tools.
  - SMT solvers are installed by scripts/install-smt-solvers.sh.
  - xmllint, FuseSoC, and Edalize are installed by
    scripts/install-core-integration-tools.sh.
  - Ubuntu 24.04 commonly provides iverilog, yosys, and verilator via apt, but
    may not provide a SymbiYosys/sby package. Use --with-sby-source when an
    explicit upstream source install is acceptable for the host.
USAGE
}

YES=0
DRY_RUN=0
SKIP_SBY=0
WITH_SBY_SOURCE=0
SBY_REF=""
PREFIX="/usr/local"

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
    --skip-sby)
      SKIP_SBY=1
      shift
      ;;
    --with-sby-source)
      WITH_SBY_SOURCE=1
      shift
      ;;
    --sby-ref)
      if (($# < 2)); then
        echo "error: --sby-ref requires a value" >&2
        exit 2
      fi
      SBY_REF="$2"
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
need_command apt-cache

apt_package_available() {
  apt-cache show "$1" >/dev/null 2>&1
}

apt_packages=(iverilog yosys verilator)
for candidate in nextpnr-ice40 nextpnr-ecp5 nextpnr-gowin; do
  if apt_package_available "${candidate}"; then
    apt_packages+=("${candidate}")
  fi
done
sby_package=""
for candidate in symbiyosys sby python3-symbiyosys; do
  if apt_package_available "${candidate}"; then
    sby_package="${candidate}"
    break
  fi
done

if [[ -n "${sby_package}" ]]; then
  apt_packages+=("${sby_package}")
elif ((SKIP_SBY == 0 && WITH_SBY_SOURCE == 1)); then
  apt_packages+=(ca-certificates git make)
elif ((SKIP_SBY == 0)); then
  cat >&2 <<'EOF'
error: no apt package providing sby/SymbiYosys was found.

Rerun with one of:
  scripts/install-oss-hdl-tools.sh --with-sby-source --yes
  scripts/install-oss-hdl-tools.sh --skip-sby --yes

The first option installs sby from https://github.com/YosysHQ/sby.git.
The second installs/checks iverilog, vvp, yosys, and verilator only.
EOF
  exit 5
fi

if ((YES == 0 && DRY_RUN == 0)); then
  echo "This will install apt packages: ${apt_packages[*]}"
  if ((WITH_SBY_SOURCE == 1 && SKIP_SBY == 0)) && [[ -z "${sby_package}" ]]; then
    echo "It will also install sby from upstream source into: ${PREFIX}"
  fi
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

tmpdir=""
cleanup() {
  if [[ -n "${tmpdir}" && -d "${tmpdir}" ]]; then
    rm -rf "${tmpdir}"
  fi
}
trap cleanup EXIT

if ((SKIP_SBY == 0 && WITH_SBY_SOURCE == 1)) && [[ -z "${sby_package}" ]]; then
  need_command git
  need_command make
  tmpdir="$(mktemp -d)"
  run git clone --depth 1 https://github.com/YosysHQ/sby.git "${tmpdir}/sby"
  if [[ -n "${SBY_REF}" ]]; then
    run git -C "${tmpdir}/sby" fetch --depth 1 origin "${SBY_REF}"
    run git -C "${tmpdir}/sby" checkout FETCH_HEAD
  fi
  run "${sudo_prefix[@]}" make -C "${tmpdir}/sby" install "PREFIX=${PREFIX}"
fi

if ((DRY_RUN == 1)); then
  echo "dry-run complete; no packages were installed and no version checks were executed."
  exit 0
fi

check_version() {
  local tool="$1"
  shift
  if command -v "${tool}" >/dev/null 2>&1; then
    echo "ok: ${tool}"
    "${tool}" "$@" | sed -n '1,3p'
  else
    echo "missing: ${tool}" >&2
    return 1
  fi
}

check_version iverilog -V
check_version vvp -V
check_version yosys -V
if command -v nextpnr-ice40 >/dev/null 2>&1; then
  check_version nextpnr-ice40 --version
else
  echo "skipped: nextpnr-ice40 package not available or not installed"
fi
if command -v nextpnr-ecp5 >/dev/null 2>&1; then
  check_version nextpnr-ecp5 --version
else
  echo "skipped: nextpnr-ecp5 package not available or not installed"
fi
if command -v nextpnr-gowin >/dev/null 2>&1; then
  check_version nextpnr-gowin --version
else
  echo "skipped: nextpnr-gowin package not available or not installed"
fi
check_version verilator --version

if ((SKIP_SBY == 0)); then
  check_version sby --version
else
  echo "skipped: sby"
fi

echo "OSS HDL tool installation check completed."
