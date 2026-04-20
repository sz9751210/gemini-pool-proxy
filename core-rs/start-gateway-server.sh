#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

TOOLCHAIN_BIN="${HOME}/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
RUSTC_BIN="${TOOLCHAIN_BIN}/rustc"
RUSTDOC_BIN="${TOOLCHAIN_BIN}/rustdoc"
CARGO_BIN="${TOOLCHAIN_BIN}/cargo"

if [[ ! -x "${CARGO_BIN}" ]]; then
  echo "[ERROR] stable toolchain cargo not found at ${CARGO_BIN}" >&2
  exit 1
fi

exec env RUSTC="${RUSTC_BIN}" RUSTDOC="${RUSTDOC_BIN}" "${CARGO_BIN}" run -p gateway-server
