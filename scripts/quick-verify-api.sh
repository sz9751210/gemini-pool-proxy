#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
PROXY_TOKEN="${PROXY_TOKEN:-sk-user-123456}"
OPENAI_MODEL="${OPENAI_MODEL:-gemini-2.5-flash}"
ALIAS_MODEL="${ALIAS_MODEL:-claude-sonnet}"
NATIVE_MODEL="${NATIVE_MODEL:-gemini-2.5-flash}"

HAS_JQ=0
if command -v jq >/dev/null 2>&1; then
  HAS_JQ=1
fi

PASSED=0
FAILED=0
WARNED=0

print_body() {
  local file="$1"
  if [[ "${HAS_JQ}" -eq 1 ]]; then
    jq . "${file}" 2>/dev/null || cat "${file}"
  else
    cat "${file}"
  fi
}

run_check() {
  local name="$1"
  local expected_csv="${2:-200}"
  shift 2

  local body_file
  body_file="$(mktemp)"
  local code
  code="$(curl -sS --connect-timeout 10 --max-time 120 -o "${body_file}" -w "%{http_code}" "$@")"

  echo ""
  echo "==> ${name}"
  echo "HTTP ${code}"
  print_body "${body_file}"

  local matched=0
  IFS=',' read -r -a expected_codes <<<"${expected_csv}"
  for expected in "${expected_codes[@]}"; do
    if [[ "${code}" == "${expected}" ]]; then
      matched=1
      break
    fi
  done

  if [[ "${matched}" -eq 1 ]]; then
    if [[ "${code}" == "${expected_codes[0]}" ]]; then
      PASSED=$((PASSED + 1))
    else
      WARNED=$((WARNED + 1))
      echo "[warn] accepted non-primary status ${code} for ${name}"
    fi
  else
    FAILED=$((FAILED + 1))
  fi

  rm -f "${body_file}"
}

echo "================================================="
echo "Gemini Pool Proxy Quick Verify"
echo "BASE_URL=${BASE_URL}"
echo "OPENAI_MODEL=${OPENAI_MODEL}"
echo "ALIAS_MODEL=${ALIAS_MODEL}"
echo "NATIVE_MODEL=${NATIVE_MODEL}"
echo "================================================="

run_check "1) /v1/models (Bearer auth)" 200 \
  "${BASE_URL}/v1/models" \
  -H "Authorization: Bearer ${PROXY_TOKEN}"

run_check "2) /v1/chat/completions (real model)" "200,429,503" \
  "${BASE_URL}/v1/chat/completions" \
  -H "Authorization: Bearer ${PROXY_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"${OPENAI_MODEL}\",\"messages\":[{\"role\":\"user\",\"content\":\"Reply with QUICK_VERIFY_OK only\"}]}"

run_check "3) /v1/chat/completions (alias model)" "200,429,503" \
  "${BASE_URL}/v1/chat/completions" \
  -H "Authorization: Bearer ${PROXY_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"${ALIAS_MODEL}\",\"messages\":[{\"role\":\"user\",\"content\":\"Reply with ALIAS_VERIFY_OK only\"}]}"

run_check "4) /v1beta native (X-goog-api-key)" "200,429,503" \
  "${BASE_URL}/v1beta/models/${NATIVE_MODEL}:generateContent" \
  -H "X-goog-api-key: ${PROXY_TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{\"contents\":[{\"parts\":[{\"text\":\"ping\"}]}]}"

echo ""
echo "================================================="
echo "Result: ${PASSED} passed, ${WARNED} warning, ${FAILED} failed"
echo "================================================="

if [[ "${FAILED}" -gt 0 ]]; then
  exit 1
fi
