#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VITE_DEV_PORT="${VITE_DEV_PORT:-1420}"
RUNTIME_PORT_START="${RUNTIME_PORT_START:-18080}"
RUNTIME_PORT_END="${RUNTIME_PORT_END:-18099}"
START_LOCK_DIR="${TMPDIR:-/tmp}/gemini-balance-desktop-start.lock"
START_LOCK_PID_FILE="${START_LOCK_DIR}/pid"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required but not found in PATH" >&2
  exit 1
fi

if ! command -v lsof >/dev/null 2>&1; then
  echo "lsof is required but not found in PATH" >&2
  exit 1
fi

is_port_listening() {
  local port="$1"
  lsof -nP -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1
}

read_desktop_pids() {
  ps -Ao pid=,command= \
    | awk '/(^|\/)gemini-balance-desktop([[:space:]]|$)/ {print $1}' \
    | sort -u
}

acquire_start_lock() {
  if mkdir "${START_LOCK_DIR}" >/dev/null 2>&1; then
    echo "$$" > "${START_LOCK_PID_FILE}"
    return 0
  fi

  local owner_pid=""
  if [[ -f "${START_LOCK_PID_FILE}" ]]; then
    owner_pid="$(cat "${START_LOCK_PID_FILE}" 2>/dev/null || true)"
  fi

  if [[ -n "${owner_pid}" ]] && kill -0 "${owner_pid}" >/dev/null 2>&1; then
    local owner_cmd
    owner_cmd="$(ps -p "${owner_pid}" -o command= 2>/dev/null | xargs || true)"
    if [[ "${owner_cmd}" == *"start-desktop.sh"* ]]; then
      echo "Found another start-desktop.sh running (PID ${owner_pid}). Automatically closing it..." >&2
      # 終止其所有子進程（Tauri/Node等），再殺死主進程
      pkill -P "${owner_pid}" >/dev/null 2>&1 || true
      kill -9 "${owner_pid}" >/dev/null 2>&1 || true
      sleep 1
    fi
  fi

  rm -rf "${START_LOCK_DIR}" >/dev/null 2>&1 || true
  if ! mkdir "${START_LOCK_DIR}" >/dev/null 2>&1; then
    echo "Failed to acquire start lock: ${START_LOCK_DIR}" >&2
    exit 1
  fi
  echo "$$" > "${START_LOCK_PID_FILE}"
}

cleanup_start_lock() {
  local owner_pid=""
  if [[ -f "${START_LOCK_PID_FILE}" ]]; then
    owner_pid="$(cat "${START_LOCK_PID_FILE}" 2>/dev/null || true)"
  fi
  if [[ "${owner_pid}" == "$$" ]]; then
    rm -rf "${START_LOCK_DIR}" >/dev/null 2>&1 || true
  fi
}

ensure_single_desktop_instance() {
  local pids
  pids=($(read_desktop_pids))
  if [[ ${#pids[@]} -eq 0 ]]; then
    echo "[0/3] Instance check: no running gemini-balance-desktop process found"
    return 0
  fi

  echo "[0/3] Instance check: found ${#pids[@]} running gemini-balance-desktop process(es), stopping..."
  local pid
  for pid in "${pids[@]}"; do
    [[ -z "${pid}" ]] && continue
    echo "  - stopping PID ${pid}"
    kill "${pid}" >/dev/null 2>&1 || true
  done

  sleep 1
  pids=($(read_desktop_pids))
  if [[ ${#pids[@]} -gt 0 ]]; then
    echo "  - force stopping remaining process(es): ${pids[*]}"
    for pid in "${pids[@]}"; do
      [[ -z "${pid}" ]] && continue
      kill -9 "${pid}" >/dev/null 2>&1 || true
    done
    sleep 1
  fi

  pids=($(read_desktop_pids))
  if [[ ${#pids[@]} -gt 0 ]]; then
    echo "Failed to ensure single desktop instance. Remaining PIDs: ${pids[*]}" >&2
    exit 1
  fi
}

release_dev_port_if_needed() {
  local port="$1"
  if ! is_port_listening "${port}"; then
    return 0
  fi

  echo "[0/3] Port check: ${port} is occupied, trying to release stale dev processes..."
  local pids
  pids=($(lsof -t -nP -iTCP:"${port}" -sTCP:LISTEN | sort -u))
  for pid in "${pids[@]}"; do
    [[ -z "${pid}" ]] && continue
    local cmd
    cmd="$(ps -p "${pid}" -o comm= | xargs)"
    echo "  - found PID ${pid} (${cmd})"
    if [[ "${cmd}" =~ (node|npm|vite|cargo|gemini-balance-desktop) ]]; then
      kill "${pid}" >/dev/null 2>&1 || true
    else
      echo "Port ${port} is occupied by a non-managed process (${cmd}, PID ${pid})." >&2
      echo "Please stop it manually and rerun ./start-desktop.sh" >&2
      exit 1
    fi
  done

  sleep 1
  if is_port_listening "${port}"; then
    echo "  - force killing remaining listeners on ${port}..."
    pids=($(lsof -t -nP -iTCP:"${port}" -sTCP:LISTEN | sort -u))
    for pid in "${pids[@]}"; do
      [[ -z "${pid}" ]] && continue
      kill -9 "${pid}" >/dev/null 2>&1 || true
    done
    sleep 1
  fi

  if is_port_listening "${port}"; then
    echo "Failed to free port ${port}. Please release it manually." >&2
    exit 1
  fi
}

ensure_runtime_port_range_available() {
  local start="$1"
  local end="$2"
  local free_port=""
  local port

  for ((port=start; port<=end; port++)); do
    if ! is_port_listening "${port}"; then
      free_port="${port}"
      break
    fi
  done

  if [[ -z "${free_port}" ]]; then
    echo "[0/3] Port check failed: no free ports in ${start}-${end}" >&2
    echo "Please release at least one port in that range and rerun." >&2
    exit 1
  fi

  echo "[0/3] Port check: runtime range ${start}-${end} OK (first free: ${free_port})"
}

if RUSTC_PATH="$(rustup which --toolchain stable rustc 2>/dev/null)"; then
  :
else
  RUSTC_PATH="$(rustup which rustc)"
fi

TOOLCHAIN_BIN="$(dirname "${RUSTC_PATH}")"
RUSTC_BIN="${TOOLCHAIN_BIN}/rustc"
RUSTDOC_BIN="${TOOLCHAIN_BIN}/rustdoc"
CARGO_BIN="${TOOLCHAIN_BIN}/cargo"
PATH_WITH_TOOLCHAIN="${TOOLCHAIN_BIN}:${PATH}"

if [[ ! -x "${CARGO_BIN}" || ! -x "${RUSTC_BIN}" ]]; then
  echo "[Error] Failed to locate rustup toolchain binaries." >&2
  echo "Expected cargo at: ${CARGO_BIN}" >&2
  echo "Expected rustc at: ${RUSTC_BIN}" >&2
  exit 1
fi

if ! RUSTC_INFO="$("${RUSTC_BIN}" -vV 2>&1)"; then
  echo "[Error] Selected rustc is unhealthy: ${RUSTC_BIN}" >&2
  echo "${RUSTC_INFO}" >&2
  echo "[Hint] This is commonly caused by Rust/LLVM mismatch (for example Homebrew rust + llvm mismatch)." >&2
  echo "[Hint] Please run: rustup toolchain install stable && rustup default stable" >&2
  exit 1
fi
RUSTC_VERSION="$(printf '%s\n' "${RUSTC_INFO}" | head -n 1)"

if ! CARGO_VERSION="$("${CARGO_BIN}" -V 2>&1)"; then
  echo "[Error] Selected cargo is unhealthy: ${CARGO_BIN}" >&2
  echo "${CARGO_VERSION}" >&2
  exit 1
fi

echo "[0/3] Rust toolchain check: ${RUSTC_VERSION}"
echo "[0/3] Cargo check: ${CARGO_VERSION}"

acquire_start_lock
trap cleanup_start_lock EXIT INT TERM

ensure_single_desktop_instance
release_dev_port_if_needed "${VITE_DEV_PORT}"
ensure_runtime_port_range_available "${RUNTIME_PORT_START}" "${RUNTIME_PORT_END}"
echo "[info] API mode: v1-only (/api/v1/*, /v1/*). Any /api/v2/* or /v2/* requests will return 410 Gone."

echo "[1/3] Building Rust gateway-server..."
(
  cd "${ROOT_DIR}/core-rs"
  env RUSTC="${RUSTC_BIN}" RUSTDOC="${RUSTDOC_BIN}" "${CARGO_BIN}" build -p gateway-server
)

echo "[2/3] Installing desktop frontend dependencies..."
(
  cd "${ROOT_DIR}/desktop/tauri-app"
  npm install --no-audit --no-fund
)

echo "[3/3] Starting Tauri desktop app..."
(
  cd "${ROOT_DIR}/desktop/tauri-app"
  PATH="${PATH_WITH_TOOLCHAIN}" \
  RUSTC="${RUSTC_BIN}" \
  RUSTDOC="${RUSTDOC_BIN}" \
  CARGO="${CARGO_BIN}" \
  GATEWAY_SERVER_BIN="${ROOT_DIR}/core-rs/target/debug/gateway-server" \
  npm run tauri:dev
)
