#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

usage() {
  cat <<'USAGE'
Install SMT solvers used by af formal flows.

Installs/checks:
  - boolector
  - z3
  - yices-smt2
  - bitwuzla
  - cvc5

Supported host package manager:
  - apt on Debian/Ubuntu

Usage:
  scripts/install-smt-solvers.sh [options]

Options:
  --yes                       Do not prompt before installation.
  --dry-run                   Print actions without changing the host.
  --allow-missing             Do not fail when a distro does not package a solver.
  --with-yices-ppa            On Ubuntu, add ppa:sri-csl/formal-methods for yices2.
  --with-yices-binary         Install Yices from the official Linux x86_64 tarball
                              when no apt package is available.
  --with-bitwuzla-source      Build and install Bitwuzla from upstream source when
                              no apt package is available.
  --bitwuzla-ref <ref>        Git tag/branch for --with-bitwuzla-source.
                              Default: 0.8.2
  --yices-url <url>           Yices tarball URL for --with-yices-binary.
  --yices-sha256 <sha256>     Expected sha256 for --yices-url.
  --prefix <path>             Install prefix for source/binary installs.
                              Default: /usr/local
  --help                      Show this help.

Examples:
  scripts/install-smt-solvers.sh --yes
  scripts/install-smt-solvers.sh --yes --with-yices-ppa --allow-missing
  scripts/install-smt-solvers.sh --yes --with-yices-binary --with-bitwuzla-source
  scripts/install-smt-solvers.sh --dry-run --with-yices-binary --with-bitwuzla-source

Notes:
  - This script does not install vendor EDA tools.
  - yices-smt2 and bitwuzla are not packaged by every Debian/Ubuntu release.
    Use the explicit source/binary flags when host-local upstream installs are
    acceptable.
USAGE
}

YES=0
DRY_RUN=0
ALLOW_MISSING=0
WITH_YICES_PPA=0
WITH_YICES_BINARY=0
WITH_BITWUZLA_SOURCE=0
BITWUZLA_REF="0.8.2"
PREFIX="/usr/local"
YICES_URL="https://yices.csl.sri.com/releases/2.6.4/yices-2.6.4-x86_64-pc-linux-gnu.tar.gz"
YICES_SHA256="841184509aecdc4df99c7ee280e33f76359032dc367919260a916257229601a4"

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
    --allow-missing)
      ALLOW_MISSING=1
      shift
      ;;
    --with-yices-ppa)
      WITH_YICES_PPA=1
      shift
      ;;
    --with-yices-binary)
      WITH_YICES_BINARY=1
      shift
      ;;
    --with-bitwuzla-source)
      WITH_BITWUZLA_SOURCE=1
      shift
      ;;
    --bitwuzla-ref)
      if (($# < 2)); then
        echo "error: --bitwuzla-ref requires a value" >&2
        exit 2
      fi
      BITWUZLA_REF="$2"
      shift 2
      ;;
    --yices-url)
      if (($# < 2)); then
        echo "error: --yices-url requires a value" >&2
        exit 2
      fi
      YICES_URL="$2"
      shift 2
      ;;
    --yices-sha256)
      if (($# < 2)); then
        echo "error: --yices-sha256 requires a value" >&2
        exit 2
      fi
      YICES_SHA256="$2"
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

apt_packages=()
missing_tools=()

add_package() {
  local package="$1"
  local item
  for item in "${apt_packages[@]}"; do
    if [[ "${item}" == "${package}" ]]; then
      return
    fi
  done
  apt_packages+=("${package}")
}

select_apt_package() {
  local tool="$1"
  shift
  local candidate
  for candidate in "$@"; do
    if apt_package_available "${candidate}"; then
      add_package "${candidate}"
      return 0
    fi
  done
  missing_tools+=("${tool}")
  return 1
}

if ((WITH_YICES_PPA == 1)); then
  case "${ID:-}" in
    ubuntu)
      add_package ca-certificates
      add_package software-properties-common
      ;;
    *)
      echo "error: --with-yices-ppa is supported only on Ubuntu hosts" >&2
      exit 4
      ;;
  esac
fi

run "${sudo_prefix[@]}" apt-get update

if ((WITH_YICES_PPA == 1)); then
  run "${sudo_prefix[@]}" apt-get install -y --no-install-recommends ca-certificates software-properties-common
  run "${sudo_prefix[@]}" add-apt-repository -y ppa:sri-csl/formal-methods
  run "${sudo_prefix[@]}" apt-get update
fi

select_apt_package boolector boolector || true
select_apt_package z3 z3 || true
select_apt_package cvc5 cvc5 || true

yices_has_package=0
if select_apt_package yices-smt2 yices2 yices; then
  yices_has_package=1
fi

bitwuzla_has_package=0
if select_apt_package bitwuzla bitwuzla; then
  bitwuzla_has_package=1
fi

if ((WITH_YICES_BINARY == 1 && yices_has_package == 0)); then
  add_package ca-certificates
  add_package curl
  add_package tar
fi

if ((WITH_BITWUZLA_SOURCE == 1 && bitwuzla_has_package == 0)); then
  add_package build-essential
  add_package ca-certificates
  add_package git
  add_package libcadical-dev
  add_package libgmp-dev
  add_package libmpfr-dev
  add_package libsymfpu-dev
  add_package m4
  add_package meson
  add_package ninja-build
  add_package pkg-config
  add_package python3
fi

if ((ALLOW_MISSING == 0)); then
  unresolved=()
  for tool in "${missing_tools[@]}"; do
    case "${tool}" in
      yices-smt2)
        if ((WITH_YICES_BINARY == 0)); then
          unresolved+=("${tool}")
        fi
        ;;
      bitwuzla)
        if ((WITH_BITWUZLA_SOURCE == 0)); then
          unresolved+=("${tool}")
        fi
        ;;
      *)
        unresolved+=("${tool}")
        ;;
    esac
  done
  if ((${#unresolved[@]} > 0)); then
    cat >&2 <<EOF
error: no apt package path found for: ${unresolved[*]}

Rerun with one of:
  scripts/install-smt-solvers.sh --with-yices-ppa --yes
  scripts/install-smt-solvers.sh --with-yices-binary --with-bitwuzla-source --yes
  scripts/install-smt-solvers.sh --allow-missing --yes
EOF
    exit 5
  fi
fi

if ((YES == 0 && DRY_RUN == 0)); then
  echo "This will install apt packages: ${apt_packages[*]}"
  if ((WITH_YICES_BINARY == 1 && yices_has_package == 0)); then
    echo "It will also install yices-smt2 from: ${YICES_URL}"
  fi
  if ((WITH_BITWUZLA_SOURCE == 1 && bitwuzla_has_package == 0)); then
    echo "It will also build bitwuzla from https://github.com/bitwuzla/bitwuzla at ref: ${BITWUZLA_REF}"
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

if ((${#apt_packages[@]} > 0)); then
  run "${sudo_prefix[@]}" apt-get install -y --no-install-recommends "${apt_packages[@]}"
fi

if ((DRY_RUN == 1)); then
  if ((WITH_YICES_BINARY == 1 && yices_has_package == 0)); then
    echo "would install yices-smt2 from: ${YICES_URL}"
  fi
  if ((WITH_BITWUZLA_SOURCE == 1 && bitwuzla_has_package == 0)); then
    echo "would build bitwuzla from https://github.com/bitwuzla/bitwuzla at ref: ${BITWUZLA_REF}"
  fi
  echo "dry-run complete; no packages were installed and no version checks were executed."
  exit 0
fi

tmpdir=""
cleanup() {
  if [[ -n "${tmpdir}" && -d "${tmpdir}" ]]; then
    rm -rf "${tmpdir}"
  fi
}
trap cleanup EXIT

tmpdir="$(mktemp -d)"

if ((WITH_YICES_BINARY == 1 && yices_has_package == 0)); then
  need_command curl
  need_command tar
  archive="${tmpdir}/yices.tar.gz"
  run curl -fsSL "${YICES_URL}" -o "${archive}"
  if [[ -n "${YICES_SHA256}" ]]; then
    printf '%s  %s\n' "${YICES_SHA256}" "${archive}" | sha256sum -c -
  fi
  run tar -xzf "${archive}" -C "${tmpdir}"
  yices_dir="$(find "${tmpdir}" -maxdepth 1 -type d -name 'yices-*' | sort | head -n 1)"
  if [[ -z "${yices_dir}" || ! -x "${yices_dir}/install-yices" ]]; then
    echo "error: Yices archive did not contain an executable install-yices script" >&2
    exit 6
  fi
  pushd "${yices_dir}" >/dev/null
  run "${sudo_prefix[@]}" ./install-yices "${PREFIX}"
  popd >/dev/null
fi

if ((WITH_BITWUZLA_SOURCE == 1 && bitwuzla_has_package == 0)); then
  need_command git
  need_command ninja
  bitwuzla_dir="${tmpdir}/bitwuzla"
  run git clone --depth 1 --branch "${BITWUZLA_REF}" https://github.com/bitwuzla/bitwuzla.git "${bitwuzla_dir}"
  pushd "${bitwuzla_dir}" >/dev/null
  run python3 ./configure.py --prefix "${PREFIX}"
  if [[ ! -f build/build.ninja ]]; then
    echo "error: Bitwuzla configure did not create build/build.ninja" >&2
    exit 7
  fi
  run "${sudo_prefix[@]}" ninja -C build install
  popd >/dev/null
fi

check_version() {
  local tool="$1"
  shift
  if command -v "${tool}" >/dev/null 2>&1; then
    echo "ok: ${tool}"
    "${tool}" "$@" | sed -n '1,3p'
  else
    if ((ALLOW_MISSING == 1)); then
      echo "missing: ${tool} (allowed)"
    else
      echo "missing: ${tool}" >&2
      return 1
    fi
  fi
}

check_version boolector --version
check_version z3 --version
check_version yices-smt2 --version
check_version bitwuzla --version
check_version cvc5 --version

echo "SMT solver installation check completed."
