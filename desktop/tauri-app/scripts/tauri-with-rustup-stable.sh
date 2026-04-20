#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: tauri-with-rustup-stable.sh <dev|build> [tauri args...]" >&2
  exit 2
fi

TAURI_SUBCOMMAND="$1"
shift

if [[ "${TAURI_SUBCOMMAND}" != "dev" && "${TAURI_SUBCOMMAND}" != "build" ]]; then
  echo "unsupported tauri subcommand: ${TAURI_SUBCOMMAND}" >&2
  exit 2
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required but not found in PATH" >&2
  exit 1
fi

if RUSTC_PATH="$(rustup which --toolchain stable rustc 2>/dev/null)"; then
  :
else
  RUSTC_PATH="$(rustup which rustc)"
fi

TOOLCHAIN_BIN="$(dirname "${RUSTC_PATH}")"
export PATH="${TOOLCHAIN_BIN}:${PATH}"
export RUSTC="${TOOLCHAIN_BIN}/rustc"
export RUSTDOC="${TOOLCHAIN_BIN}/rustdoc"
export CARGO="${TOOLCHAIN_BIN}/cargo"

if [[ ! -x "${CARGO}" || ! -x "${RUSTC}" ]]; then
  echo "failed to locate rustup toolchain binaries." >&2
  echo "expected cargo at: ${CARGO}" >&2
  echo "expected rustc at: ${RUSTC}" >&2
  exit 1
fi

if ! RUSTC_INFO="$("${RUSTC}" -vV 2>&1)"; then
  echo "selected rustc is unhealthy: ${RUSTC}" >&2
  echo "${RUSTC_INFO}" >&2
  echo "hint: this is commonly caused by Rust/LLVM mismatch (for example Homebrew rust + llvm mismatch)." >&2
  echo "hint: run rustup toolchain install stable && rustup default stable" >&2
  exit 1
fi
RUSTC_VERSION="$(printf '%s\n' "${RUSTC_INFO}" | head -n 1)"

if ! CARGO_VERSION="$("${CARGO}" -V 2>&1)"; then
  echo "selected cargo is unhealthy: ${CARGO}" >&2
  echo "${CARGO_VERSION}" >&2
  exit 1
fi

echo "[toolchain] ${RUSTC_VERSION}" >&2
echo "[toolchain] ${CARGO_VERSION}" >&2

TAURI_BIN="$(command -v tauri || true)"
if [[ -z "${TAURI_BIN}" && -x "./node_modules/.bin/tauri" ]]; then
  TAURI_BIN="./node_modules/.bin/tauri"
fi

if [[ -z "${TAURI_BIN}" ]]; then
  echo "tauri CLI not found. run npm install in desktop/tauri-app first." >&2
  exit 1
fi

if [[ "${TAURI_SUBCOMMAND}" == "dev" ]]; then
  exec "${TAURI_BIN}" dev "$@"
fi

exec "${TAURI_BIN}" build "$@"
