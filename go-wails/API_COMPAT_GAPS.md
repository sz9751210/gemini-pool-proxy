# API Compatibility Gaps (Rust vs Go-Wails)

Date: 2026-04-22  
Scope: compare `core-rs` router (`core-rs/crates/gateway-server/src/routes.rs`) with current `go-wails` router (`go-wails/internal/server/router.go`)

## 1. Implemented in Go-Wails

### Proxy

- `GET /v1/models`
- `POST /v1/chat/completions`
- `GET|POST /v1beta/*`

### Admin v1 (core set)

- `POST /api/v1/session/login`
- `POST /api/v1/session/logout`
- `GET /api/v1/session/status`
- `GET|PUT /api/v1/config`
- `GET /api/v1/keys`
- `GET /api/v1/health`
- `POST /api/v1/keys/actions`
- `GET /api/v1/keys/usage/{key}`
- `GET /api/v1/logs`
- `GET /api/v1/dashboard/overview`
- `GET /api/v1/pool/status`
- `PUT /api/v1/pool/strategy`
- `POST /api/v1/runtime/start`
- `POST /api/v1/runtime/stop`

## 2. Missing vs Rust v1 Routes

### Keys / Stats

- `GET /api/v1/keys/all`
- `GET /api/v1/stats/details`
- `GET /api/v1/stats/attention-keys`
- `GET /api/v1/stats/key-details`

### Config management extras

- `POST /api/v1/config/reset`
- `GET /api/v1/config/schema`
- `GET /api/v1/config/ui-models`
- `POST /api/v1/config/keys/add`
- `POST /api/v1/config/keys/delete`
- `POST /api/v1/config/proxies/add`
- `POST /api/v1/config/proxies/delete`

### Proxy tools / scheduler

- `POST /api/v1/proxy/check`
- `POST /api/v1/proxy/check-all`
- `GET /api/v1/proxy/cache-stats`
- `POST /api/v1/proxy/cache-clear`
- `POST /api/v1/scheduler/start`
- `POST /api/v1/scheduler/stop`
- `GET /api/v1/scheduler/status`

### Logs detail and delete APIs

- `GET /api/v1/logs/lookup`
- `GET /api/v1/logs/{id}`
- `DELETE /api/v1/logs/{id}`
- `DELETE /api/v1/logs/bulk`
- `DELETE /api/v1/logs/all`

### Additional proxy compatibility

- `POST|GET /v1/models/{*path}`
- `/api/v2*` and `/v2*` deprecation handlers

## 3. Missing Legacy Compatibility Families

Rust has additional compatibility trees not yet present in Go-Wails:

- `/api/config/*` (legacy aliases)
- `/api/scheduler/*`
- `/api/stats/*`
- `/api/keys*` legacy aliases
- `/api/logs/errors*`
- `/gemini/v1beta/verify-key/*`
- `/api/compat/v1/*`
- `/api/pro/*`

## 4. Behavior-Level Gaps

- Error envelope parity:
  - Rust returns OpenAI-style error payloads for multiple failure classes.
  - Go-Wails still returns simpler `http.Error` plain text for many paths.
- Key model in admin:
  - Rust has richer key action workflows (including more action targets and legacy alias coverage).
  - Go-Wails now supports base `actions` and `usage` routes, but still lacks the full extended workflow surface.
- Logs query surface:
  - Rust supports lookup/detail/delete and advanced filtering/sorting.
  - Go-Wails currently supports list endpoint with basic pagination.

## 5. Recommended Priority Order

1. `GET /api/v1/logs/lookup`, `GET /api/v1/logs/{id}`, delete APIs
2. `GET /api/v1/config/schema` + config keys add/delete helpers
3. `POST /api/v1/proxy/check*` and cache APIs
4. `GET /api/v1/stats/*` family for operational analytics
5. Optional legacy aliases (`/api/config/*`, `/api/compat/v1/*`, `/api/pro/*`)

## 6. Compatibility Assessment

- Contract level for modern clients (`/v1`, `/v1beta`, core `/api/v1`) is partially aligned.
- Full parity with Rust route surface is not yet complete.
- If strict drop-in replacement is required for existing admin tooling and legacy clients, additional route migration work is still needed.
