#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
WAIT_SECONDS="${WAIT_SECONDS:-90}"
START_HEADLESS_IF_NEEDED="${START_HEADLESS_IF_NEEDED:-1}"
USE_RTK_PROXY_CURL="${USE_RTK_PROXY_CURL:-0}"

headless_pid=""
started_here=0

is_port_listening() {
  local base_url="$1"
  local host_port="${base_url#*://}"
  host_port="${host_port%%/*}"
  local host="${host_port%%:*}"
  local port="${host_port##*:}"
  if [[ "${host}" == "${host_port}" ]]; then
    host="127.0.0.1"
    port="18080"
  fi
  lsof -nP -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1
}

cleanup() {
  if [[ "${started_here}" -eq 1 ]] && [[ -n "${headless_pid}" ]]; then
    kill "${headless_pid}" >/dev/null 2>&1 || true
    wait "${headless_pid}" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

if is_port_listening "${BASE_URL}"; then
  echo "[info] detected existing server for ${BASE_URL}, will reuse it"
else
  if [[ "${START_HEADLESS_IF_NEEDED}" != "1" ]]; then
    echo "[error] server not ready at ${BASE_URL} and START_HEADLESS_IF_NEEDED=0" >&2
    exit 1
  fi

  echo "[info] server not ready, starting ./start-headless.sh"
  (
    cd "${ROOT_DIR}"
    ./start-headless.sh
  ) &
  headless_pid="$!"
  started_here=1

  echo "[info] waiting for server to listen on ${BASE_URL} (timeout: ${WAIT_SECONDS}s)"
  for _ in $(seq 1 "${WAIT_SECONDS}"); do
    if is_port_listening "${BASE_URL}"; then
      echo "[info] server is ready"
      break
    fi
    sleep 1
  done

  if ! is_port_listening "${BASE_URL}"; then
    echo "[error] server failed to start within ${WAIT_SECONDS}s" >&2
    exit 1
  fi
fi

echo "[info] running per-key-cycle verification"
(
  cd "${ROOT_DIR}"
  BASE_URL="${BASE_URL}" USE_RTK_PROXY_CURL="${USE_RTK_PROXY_CURL}" \
    ./scripts/verify-per-key-cycle.sh
)

echo "[info] verification completed"
