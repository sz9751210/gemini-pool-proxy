#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNTIME_PORT_START="${RUNTIME_PORT_START:-}"
RUNTIME_PORT_END="${RUNTIME_PORT_END:-}"

echo "================================================="
echo "🚀 Starting Gemini Pool Proxy (Headless Mode)"
echo "================================================="
echo "[Info] Environment variables will be loaded from .env"
echo "[Info] Press Ctrl+C to gracefully stop the server."
echo ""

cd "${ROOT_DIR}/core-rs"

# Ensure rustup is available
if ! command -v rustup >/dev/null 2>&1; then
  echo "[Error] rustup is required but not found in PATH." >&2
  echo "Please install Rust from https://rustup.rs/ and try again." >&2
  exit 1
fi

if ! command -v lsof >/dev/null 2>&1; then
  echo "[Error] lsof is required but not found in PATH." >&2
  echo "Please install lsof and try again." >&2
  exit 1
fi

is_port_listening() {
  local port="$1"
  lsof -nP -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1
}

read_dotenv_value() {
  local key="$1"
  local dotenv_file="${ROOT_DIR}/.env"
  [[ -f "${dotenv_file}" ]] || return 0

  local value
  value="$(awk -F= -v target="${key}" '
    /^[[:space:]]*#/ { next }
    $0 !~ /=/ { next }
    {
      current_key = $1
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", current_key)
      if (current_key != target) {
        next
      }
      current_value = substr($0, index($0, "=") + 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", current_value)
      print current_value
      exit
    }
  ' "${dotenv_file}")"

  if [[ -z "${value}" ]]; then
    return 0
  fi

  if [[ "${value}" == \"*\" && "${value}" == *\" ]]; then
    value="${value:1:${#value}-2}"
  elif [[ "${value}" == \'*\' && "${value}" == *\' ]]; then
    value="${value:1:${#value}-2}"
  fi

  printf '%s\n' "${value}"
}

resolve_runtime_port_range() {
  if [[ -z "${RUNTIME_PORT_START}" ]]; then
    RUNTIME_PORT_START="$(read_dotenv_value "RUNTIME_PORT_START")"
  fi
  if [[ -z "${RUNTIME_PORT_END}" ]]; then
    RUNTIME_PORT_END="$(read_dotenv_value "RUNTIME_PORT_END")"
  fi

  RUNTIME_PORT_START="${RUNTIME_PORT_START:-18080}"
  RUNTIME_PORT_END="${RUNTIME_PORT_END:-18099}"

  if ! [[ "${RUNTIME_PORT_START}" =~ ^[0-9]+$ && "${RUNTIME_PORT_END}" =~ ^[0-9]+$ ]]; then
    echo "[Error] Invalid runtime port range: ${RUNTIME_PORT_START}-${RUNTIME_PORT_END}" >&2
    exit 1
  fi

  if ((RUNTIME_PORT_START < 1 || RUNTIME_PORT_START > 65535)); then
    echo "[Error] RUNTIME_PORT_START must be between 1 and 65535: ${RUNTIME_PORT_START}" >&2
    exit 1
  fi
  if ((RUNTIME_PORT_END < 1 || RUNTIME_PORT_END > 65535)); then
    echo "[Error] RUNTIME_PORT_END must be between 1 and 65535: ${RUNTIME_PORT_END}" >&2
    exit 1
  fi
  if ((RUNTIME_PORT_START > RUNTIME_PORT_END)); then
    echo "[Error] RUNTIME_PORT_START cannot be greater than RUNTIME_PORT_END: ${RUNTIME_PORT_START}-${RUNTIME_PORT_END}" >&2
    exit 1
  fi

  echo "[Info] Runtime port range: ${RUNTIME_PORT_START}-${RUNTIME_PORT_END}"
}

release_runtime_port_if_needed() {
  local port="$1"
  if ! is_port_listening "${port}"; then
    return 0
  fi

  echo "[Info] Port check: ${port} is occupied, trying to release stale processes..."
  local pids
  pids=($(lsof -t -nP -iTCP:"${port}" -sTCP:LISTEN | sort -u))
  local pid
  for pid in "${pids[@]}"; do
    [[ -z "${pid}" ]] && continue
    local cmd
    cmd="$(ps -p "${pid}" -o comm= | xargs)"
    echo "  - found PID ${pid} (${cmd})"
    if [[ "${cmd}" =~ (gateway-server|cargo|rustc|node|npm|start-headless) ]]; then
      kill "${pid}" >/dev/null 2>&1 || true
    else
      echo "[Error] Port ${port} is occupied by a non-managed process (${cmd}, PID ${pid})." >&2
      echo "[Error] Please stop it manually and rerun ./start-headless.sh" >&2
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
    echo "[Error] Failed to free port ${port}. Please release it manually." >&2
    exit 1
  fi
}

release_runtime_range_if_needed() {
  local start="$1"
  local end="$2"
  local port

  for ((port=start; port<=end; port++)); do
    release_runtime_port_if_needed "${port}"
  done
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
    echo "[Error] Port check failed: no free ports in ${start}-${end}" >&2
    echo "[Error] Please release at least one port in that range and rerun." >&2
    exit 1
  fi

  echo "[Info] Port check: runtime range ${start}-${end} OK (first free: ${free_port})"
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

echo "[Info] Using Rust toolchain: ${RUSTC_VERSION}"
echo "[Info] Using Cargo: ${CARGO_VERSION}"
resolve_runtime_port_range
release_runtime_range_if_needed "${RUNTIME_PORT_START}" "${RUNTIME_PORT_END}"
ensure_runtime_port_range_available "${RUNTIME_PORT_START}" "${RUNTIME_PORT_END}"

# Run the backend gateway-server in release mode for best performance
env RUSTC="${RUSTC_BIN}" RUSTDOC="${RUSTDOC_BIN}" "${CARGO_BIN}" run --release -p gateway-server
