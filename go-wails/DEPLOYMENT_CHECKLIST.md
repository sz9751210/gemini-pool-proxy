# Deployment Drill Checklist

Use this checklist for pre-production and production rollout drills.

## 0. Inputs

- [ ] Target host has Docker + Docker Compose v2
- [ ] Repository is on expected release commit/tag
- [ ] `.env` prepared with real `AUTH_TOKEN`, `ALLOWED_TOKENS`, `API_KEYS`

## 1. Preflight

- [ ] `rtk docker --version`
- [ ] `rtk docker compose version`
- [ ] `rtk docker compose config` (must parse without errors)
- [ ] `rtk bash -lc "cd go-wails && rtk go test ./..."` (optional build gate)

## 2. Build

- [ ] `rtk docker compose build go-wails-headless`
- [ ] Build completes successfully

## 3. Deploy

- [ ] `rtk docker compose up -d go-wails-headless`
- [ ] `rtk docker compose ps` shows container as running
- [ ] `rtk docker compose logs --tail=200 go-wails-headless` has no startup panic

## 4. Runtime Checks

- [ ] `rtk curl -sS http://127.0.0.1:18080/api/v1/health`
- [ ] `rtk curl -sS http://127.0.0.1:18080/v1/models -H "Authorization: Bearer <user_token>"`
- [ ] `rtk curl -sS http://127.0.0.1:18080/api/v1/dashboard/overview`
- [ ] `rtk curl -sS "http://127.0.0.1:18080/api/v1/logs?limit=20&offset=0"`

## 5. Smoke

- [ ] `rtk bash -lc "cd go-wails && BASE_URL=http://127.0.0.1:18080 PROXY_TOKEN=<user_token> ./scripts/smoke-phase2.sh"`
- [ ] Output includes `[smoke-phase2] done`

## 6. Rollback Drill

- [ ] `rtk docker compose stop go-wails-headless`
- [ ] Checkout previous release commit/tag
- [ ] `rtk docker compose build go-wails-headless`
- [ ] `rtk docker compose up -d go-wails-headless`
- [ ] Re-run health checks

## 7. Cleanup

- [ ] `rtk docker compose down` (if drill environment)
- [ ] Archive logs and command outputs
- [ ] Record pass/fail + action items
