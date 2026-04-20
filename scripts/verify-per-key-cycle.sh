#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOTENV_FILE="${DOTENV_FILE:-${ROOT_DIR}/.env}"

BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
MODEL_ALIAS="${MODEL_ALIAS:-claude-sonnet}"
REQUEST_COUNT="${REQUEST_COUNT:-8}"
CHAT_PATH="${CHAT_PATH:-/v1/chat/completions}"
USE_RTK_PROXY_CURL="${USE_RTK_PROXY_CURL:-0}"

print_error() {
  echo "[error] $*" >&2
}

require_command() {
  local cmd="$1"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    print_error "Missing required command: ${cmd}"
    exit 1
  fi
}

read_dotenv_value() {
  local key="$1"
  [[ -f "${DOTENV_FILE}" ]] || return 0

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
  ' "${DOTENV_FILE}")"

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

first_allowed_token() {
  local raw
  raw="$(read_dotenv_value "ALLOWED_TOKENS")"
  if [[ -z "${raw}" ]]; then
    return 0
  fi
  printf '%s' "${raw}" | jq -r '.[0] // empty'
}

contains_item() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    if [[ "${item}" == "${needle}" ]]; then
      return 0
    fi
  done
  return 1
}

require_command curl
require_command jq

if [[ "${USE_RTK_PROXY_CURL}" == "1" ]]; then
  require_command rtk
  CURL_CMD=(rtk proxy curl)
else
  CURL_CMD=(curl)
fi

if ! [[ "${REQUEST_COUNT}" =~ ^[0-9]+$ ]] || ((REQUEST_COUNT < 1)); then
  print_error "REQUEST_COUNT must be a positive integer, got: ${REQUEST_COUNT}"
  exit 1
fi

ADMIN_TOKEN="${ADMIN_TOKEN:-$(read_dotenv_value "AUTH_TOKEN")}"
USER_TOKEN="${USER_TOKEN:-}"
if [[ -z "${USER_TOKEN}" ]]; then
  USER_TOKEN="${PROXY_TOKEN:-}"
fi
if [[ -z "${USER_TOKEN}" ]]; then
  USER_TOKEN="$(first_allowed_token)"
fi
if [[ -z "${USER_TOKEN}" ]]; then
  USER_TOKEN="${ADMIN_TOKEN}"
fi

if [[ -z "${ADMIN_TOKEN}" ]]; then
  print_error "ADMIN_TOKEN is empty. Set ADMIN_TOKEN or AUTH_TOKEN in ${DOTENV_FILE}."
  exit 1
fi
if [[ -z "${USER_TOKEN}" ]]; then
  print_error "USER_TOKEN is empty. Set USER_TOKEN/PROXY_TOKEN/ALLOWED_TOKENS."
  exit 1
fi

echo "================================================="
echo "Per-Key-Cycle Verification"
echo "BASE_URL=${BASE_URL}"
echo "MODEL_ALIAS=${MODEL_ALIAS}"
echo "REQUEST_COUNT=${REQUEST_COUNT}"
echo "================================================="

tmp_body="$(mktemp)"
unauth_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${tmp_body}" -w "%{http_code}" "${BASE_URL}/v1/models")"
if [[ "${unauth_code}" != "401" ]]; then
  print_error "Expected /v1/models without auth to be 401, got ${unauth_code}"
  cat "${tmp_body}" >&2
  rm -f "${tmp_body}"
  exit 1
fi
rm -f "${tmp_body}"
echo "[pass] unauthorized /v1/models -> 401"

tmp_body="$(mktemp)"
auth_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${tmp_body}" -w "%{http_code}" \
  "${BASE_URL}/v1/models" \
  -H "Authorization: Bearer ${USER_TOKEN}")"
if [[ "${auth_code}" != "200" ]]; then
  print_error "Expected /v1/models with user token to be 200, got ${auth_code}"
  cat "${tmp_body}" >&2
  rm -f "${tmp_body}"
  exit 1
fi
rm -f "${tmp_body}"
echo "[pass] authorized /v1/models -> 200"

config_body="$(mktemp)"
config_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${config_body}" -w "%{http_code}" \
  "${BASE_URL}/api/v1/config" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}")"
if [[ "${config_code}" != "200" ]]; then
  print_error "Failed to fetch /api/v1/config, status=${config_code}"
  cat "${config_body}" >&2
  rm -f "${config_body}"
  exit 1
fi
config_json="$(cat "${config_body}")"
rm -f "${config_body}"

strategy="$(printf '%s' "${config_json}" | jq -r '.MODEL_POOL_STRATEGY // empty')"
if [[ "${strategy}" != "per_key_cycle" ]]; then
  print_error "MODEL_POOL_STRATEGY is ${strategy:-<empty>}, expected per_key_cycle"
  exit 1
fi
echo "[pass] MODEL_POOL_STRATEGY=per_key_cycle"

alias_models=()
while IFS= read -r model; do
  [[ -n "${model}" ]] && alias_models+=("${model}")
done < <(printf '%s' "${config_json}" | jq -r --arg alias "${MODEL_ALIAS}" '.MODEL_POOLS[$alias][]?')

if ((${#alias_models[@]} < 2)); then
  print_error "MODEL_POOLS.${MODEL_ALIAS} must contain at least 2 models"
  exit 1
fi
echo "[pass] alias ${MODEL_ALIAS} has ${#alias_models[@]} models"

reset_body="$(mktemp)"
reset_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${reset_body}" -w "%{http_code}" \
  -X POST "${BASE_URL}/api/v1/keys/actions" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"action":"reset","keyType":"all"}')"
if [[ "${reset_code}" != "200" ]]; then
  print_error "Failed to reset key failures, status=${reset_code}"
  cat "${reset_body}" >&2
  rm -f "${reset_body}"
  exit 1
fi
rm -f "${reset_body}"
echo "[pass] reset key failures"

pool_set_body="$(mktemp)"
pool_set_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${pool_set_body}" -w "%{http_code}" \
  -X PUT "${BASE_URL}/api/v1/pool/strategy" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"strategy":"round_robin"}')"
if [[ "${pool_set_code}" != "200" ]]; then
  print_error "Failed to set key pool strategy to round_robin, status=${pool_set_code}"
  cat "${pool_set_body}" >&2
  rm -f "${pool_set_body}"
  exit 1
fi
rm -f "${pool_set_body}"
echo "[pass] key pool strategy set to round_robin"

keys_body="$(mktemp)"
keys_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${keys_body}" -w "%{http_code}" \
  "${BASE_URL}/api/v1/keys/all" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}")"
if [[ "${keys_code}" != "200" ]]; then
  print_error "Failed to fetch /api/v1/keys/all, status=${keys_code}"
  cat "${keys_body}" >&2
  rm -f "${keys_body}"
  exit 1
fi
keys_json="$(cat "${keys_body}")"
rm -f "${keys_body}"
active_key_count="$(printf '%s' "${keys_json}" | jq -r '.summary.active // 0')"
if ! [[ "${active_key_count}" =~ ^[0-9]+$ ]] || ((active_key_count < 2)); then
  print_error "Need at least 2 active keys, found: ${active_key_count}"
  exit 1
fi

if ((REQUEST_COUNT < active_key_count * 2)); then
  print_error "REQUEST_COUNT (${REQUEST_COUNT}) is too small for 2 full key rounds with ${active_key_count} active keys"
  exit 1
fi
echo "[pass] active key count = ${active_key_count}"

clear_body="$(mktemp)"
clear_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${clear_body}" -w "%{http_code}" \
  -X DELETE "${BASE_URL}/api/v1/logs/all" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}")"
if [[ "${clear_code}" != "204" ]]; then
  print_error "Failed to clear logs, status=${clear_code}"
  cat "${clear_body}" >&2
  rm -f "${clear_body}"
  exit 1
fi
rm -f "${clear_body}"
echo "[pass] logs cleared"

declare -a request_codes=()
for ((i = 1; i <= REQUEST_COUNT; i++)); do
  payload="$(jq -nc --arg model "${MODEL_ALIAS}" --arg msg "per-key-cycle-check-${i}" \
    '{model:$model,messages:[{role:"user",content:$msg}],temperature:0}')"
  req_body="$(mktemp)"
  code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 25 -o "${req_body}" -w "%{http_code}" \
    "${BASE_URL}${CHAT_PATH}" \
    -H "Authorization: Bearer ${USER_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "${payload}")"
  if [[ "${code}" == "000" ]]; then
    print_error "Request #${i} failed before receiving HTTP status."
    cat "${req_body}" >&2 || true
    rm -f "${req_body}"
    exit 1
  fi
  request_codes+=("${code}")
  rm -f "${req_body}"
done
echo "[pass] sent ${REQUEST_COUNT} chat requests"

logs_body="$(mktemp)"
logs_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${logs_body}" -w "%{http_code}" \
  "${BASE_URL}/api/v1/logs?limit=${REQUEST_COUNT}&offset=0&sort_by=id&sort_order=asc" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}")"
if [[ "${logs_code}" != "200" ]]; then
  print_error "Failed to fetch logs, status=${logs_code}"
  cat "${logs_body}" >&2
  rm -f "${logs_body}"
  exit 1
fi
logs_json="$(cat "${logs_body}")"
rm -f "${logs_body}"

pool_body="$(mktemp)"
pool_code="$("${CURL_CMD[@]}" -sS --connect-timeout 5 --max-time 20 -o "${pool_body}" -w "%{http_code}" \
  "${BASE_URL}/api/v1/pool/status?limit=${REQUEST_COUNT}" \
  -H "Authorization: Bearer ${ADMIN_TOKEN}")"
if [[ "${pool_code}" != "200" ]]; then
  print_error "Failed to fetch pool status, status=${pool_code}"
  cat "${pool_body}" >&2
  rm -f "${pool_body}"
  exit 1
fi
pool_json="$(cat "${pool_body}")"
rm -f "${pool_body}"

log_models=()
while IFS= read -r item; do
  log_models+=("${item}")
done < <(printf '%s' "${logs_json}" | jq -r '.logs[].model')

log_statuses=()
while IFS= read -r item; do
  log_statuses+=("${item}")
done < <(printf '%s' "${logs_json}" | jq -r '.logs[].statusCode')

log_masked_keys=()
while IFS= read -r item; do
  log_masked_keys+=("${item}")
done < <(printf '%s' "${logs_json}" | jq -r '.logs[].maskedKey')

selected_key_ids=()
while IFS= read -r item; do
  selected_key_ids+=("${item}")
done < <(printf '%s' "${pool_json}" | jq -r '.recentSelections | reverse | .[].keyId')

selected_masked_keys=()
while IFS= read -r item; do
  selected_masked_keys+=("${item}")
done < <(printf '%s' "${pool_json}" | jq -r '.recentSelections | reverse | .[].maskedKey')

if ((${#log_models[@]} != REQUEST_COUNT)); then
  print_error "Expected ${REQUEST_COUNT} log rows, got ${#log_models[@]}"
  exit 1
fi
if ((${#selected_key_ids[@]} != REQUEST_COUNT)); then
  print_error "Expected ${REQUEST_COUNT} pool selections, got ${#selected_key_ids[@]}"
  exit 1
fi

observed_key_order=()
for key_id in "${selected_key_ids[@]}"; do
  if ! contains_item "${key_id}" "${observed_key_order[@]-}"; then
    observed_key_order+=("${key_id}")
  fi
done

if ((${#observed_key_order[@]} < active_key_count)); then
  print_error "Observed only ${#observed_key_order[@]} unique keys, expected ${active_key_count} active keys"
  exit 1
fi

echo ""
printf "%-4s %-10s %-14s %-24s %-8s %-8s\n" "#" "keyId" "maskedKey" "model" "logCode" "chatCode"
printf "%-4s %-10s %-14s %-24s %-8s %-8s\n" "----" "----------" "--------------" "------------------------" "--------" "--------"

declare -a validation_errors=()
for ((i = 0; i < REQUEST_COUNT; i++)); do
  expected_key_id="${observed_key_order[i % active_key_count]}"
  expected_model="${alias_models[(i / active_key_count) % ${#alias_models[@]}]}"

  actual_key_id="${selected_key_ids[i]}"
  actual_masked="${selected_masked_keys[i]}"
  actual_model="${log_models[i]}"
  actual_status="${log_statuses[i]}"
  request_status="${request_codes[i]}"

  if [[ "${actual_key_id}" != "${expected_key_id}" ]]; then
    validation_errors+=("request #$((i + 1)): expected key ${expected_key_id}, got ${actual_key_id}")
  fi
  if [[ "${actual_model}" != "${expected_model}" ]]; then
    validation_errors+=("request #$((i + 1)): expected model ${expected_model}, got ${actual_model}")
  fi
  if [[ "${actual_masked}" != "${log_masked_keys[i]}" ]]; then
    validation_errors+=("request #$((i + 1)): pool/log masked key mismatch (${actual_masked} vs ${log_masked_keys[i]})")
  fi

  printf "%-4s %-10s %-14s %-24s %-8s %-8s\n" \
    "$((i + 1))" "${actual_key_id}" "${actual_masked}" "${actual_model}" "${actual_status}" "${request_status}"
done

if ((${#validation_errors[@]} > 0)); then
  echo ""
  print_error "Validation failed:"
  for err in "${validation_errors[@]}"; do
    print_error "  - ${err}"
  done
  exit 1
fi

echo ""
echo "[pass] per_key_cycle validation succeeded"
echo "[pass] key sequence follows round_robin; model switches after each full key round"
echo "================================================="
