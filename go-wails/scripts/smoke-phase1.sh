#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
PROXY_TOKEN="${PROXY_TOKEN:-sk-user-demo}"

echo "[smoke] check /v1/models"
rtk curl -sS "${BASE_URL}/v1/models" -H "Authorization: Bearer ${PROXY_TOKEN}" > /tmp/go-wails-models.json

echo "[smoke] check /api/v1/health"
rtk curl -sS "${BASE_URL}/api/v1/health" > /tmp/go-wails-health.json

echo "[smoke] done"
