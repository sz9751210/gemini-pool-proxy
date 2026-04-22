#!/usr/bin/env bash
set -euo pipefail

SERVICE="${SERVICE:-go-wails-headless}"
BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
PROXY_TOKEN="${PROXY_TOKEN:-sk-user-demo}"
MODEL="${MODEL:-claude-sonnet}"
START_COMPOSE="${START_COMPOSE:-1}"
CLEANUP="${CLEANUP:-0}"
HEALTH_PATH="${HEALTH_PATH:-/api/v1/health}"
HEALTH_EXPECTED="${HEALTH_EXPECTED:-200}"
CHAT_EXPECTED="${CHAT_EXPECTED:-200,400,429,503}"

PASSED=0
FAILED=0

run_check() {
  local name="$1"
  local expected_csv="$2"
  shift 2

  local body_file
  body_file="$(mktemp)"
  local code
  code="$(curl -sS --connect-timeout 10 --max-time 120 -o "${body_file}" -w "%{http_code}" "$@")"

  echo ""
  echo "==> ${name}"
  echo "HTTP ${code}"
  cat "${body_file}"

  local matched=0
  IFS=',' read -r -a expected_codes <<<"${expected_csv}"
  for expected in "${expected_codes[@]}"; do
    if [[ "${code}" == "${expected}" ]]; then
      matched=1
      break
    fi
  done

  if [[ "${matched}" -eq 1 ]]; then
    PASSED=$((PASSED + 1))
  else
    FAILED=$((FAILED + 1))
  fi

  rm -f "${body_file}"
}

wait_for_ready() {
  local attempts=40
  local i
  for ((i=1; i<=attempts; i++)); do
    if curl -sS --connect-timeout 2 --max-time 5 "${BASE_URL}" >/dev/null 2>&1; then
      echo "[ready] ${BASE_URL}"
      return 0
    fi
    sleep 1
  done
  echo "[error] service not ready after ${attempts}s: ${BASE_URL}" >&2
  return 1
}

echo "================================================="
echo "Docker Build + API Smoke"
echo "SERVICE=${SERVICE}"
echo "BASE_URL=${BASE_URL}"
echo "MODEL=${MODEL}"
echo "START_COMPOSE=${START_COMPOSE}"
echo "CLEANUP=${CLEANUP}"
echo "HEALTH_PATH=${HEALTH_PATH}"
echo "HEALTH_EXPECTED=${HEALTH_EXPECTED}"
echo "================================================="

if [[ "${START_COMPOSE}" == "1" ]]; then
  docker compose build "${SERVICE}"
  docker compose up -d "${SERVICE}"
fi

wait_for_ready

run_check "1) ${HEALTH_PATH}" "${HEALTH_EXPECTED}" \
  "${BASE_URL}${HEALTH_PATH}"

run_check "2) /v1/models" "200" \
  "${BASE_URL}/v1/models" \
  -H "Authorization: Bearer ${PROXY_TOKEN}"

run_check "3) /v1/chat/completions" "${CHAT_EXPECTED}" \
  "${BASE_URL}/v1/chat/completions" \
  -H "Authorization: Bearer ${PROXY_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"${MODEL}\",\"messages\":[{\"role\":\"user\",\"content\":\"Reply with DOCKER_SMOKE_OK only\"}]}"

echo ""
echo "================================================="
echo "Result: ${PASSED} passed, ${FAILED} failed"
echo "================================================="

if [[ "${CLEANUP}" == "1" && "${START_COMPOSE}" == "1" ]]; then
  docker compose stop "${SERVICE}"
fi

if [[ "${FAILED}" -gt 0 ]]; then
  exit 1
fi
