#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
PROXY_TOKEN="${PROXY_TOKEN:-sk-user-demo}"

rtk curl -sS "${BASE_URL}/v1/models" -H "Authorization: Bearer ${PROXY_TOKEN}" >/tmp/go-wails-phase2-models.json
rtk curl -sS "${BASE_URL}/api/v1/dashboard/overview" >/tmp/go-wails-phase2-dashboard.json
rtk curl -sS "${BASE_URL}/api/v1/logs?limit=20&offset=0" >/tmp/go-wails-phase2-logs.json
echo "[smoke-phase2] done"
