# Release Notes

## v0.2.0

Release date: 2026-04-22

### Highlights

- Completed Go + Wails Phase 2 implementation on top of Phase 1 baseline.
- Added `keypool` strategies:
  - `round_robin`
  - `random`
  - `least_fail`
  - cooldown/failure recovery
- Added `modelpool` strategies:
  - `round_robin`
  - `least_used`
  - `per_key_cycle`
- Added metrics/logging domain for:
  - call logs
  - dashboard 24h aggregates
  - recent error snapshots
- Extended admin APIs:
  - `GET /api/v1/logs`
  - `GET /api/v1/dashboard/overview`
  - `GET /api/v1/pool/status`
  - `PUT /api/v1/pool/strategy`
- Extended Wails bindings/UI for dashboard, logs, and pool status.
- Added containerization:
  - `go-wails/Dockerfile`
  - `go-wails/.dockerignore`
  - root `docker-compose.yml`
- Added deployment documentation:
  - `go-wails/DEPLOYMENT.md`

### Verification

- `go test ./...` passed (`go-wails`).
- `npm run build` passed (`go-wails/ui/wails/frontend`).
- `scripts/smoke-phase2.sh` passed.

### Upgrade Notes

- Existing `.env` files remain compatible.
- New optional env fields are now supported and recommended:
  - `MAX_FAILURES`
  - `COOLDOWN_SECONDS`
  - `MODEL_POOL_STRATEGY`
  - `MODEL_POOL_SCOPE`

### Known Limitations

- Docker Compose runtime verification depends on local Docker daemon availability.
- Legacy compatibility routes from Rust version are not fully migrated (see `API_COMPAT_GAPS.md`).
