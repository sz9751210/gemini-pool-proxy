# Go-Wails Phase 3 Priority 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Rust-compatible Priority 1 admin APIs to Go-Wails: `POST /api/v1/keys/actions` and `GET /api/v1/keys/usage/{key}`.

**Architecture:** Extend the existing `admin` + `server` boundaries without introducing new modules. Reuse `runtime.Manager`, `keypool.Pool`, and `metrics.Store`, then add small focused methods for key actions and usage aggregation.

**Tech Stack:** Go, chi router, net/http, existing unit + integration test layout (`internal/*_test.go`, `tests/integration`).

---

### Task 1: Route Contract Coverage

**Files:**
- Modify: `go-wails/tests/integration/phase2_contract_test.go`

- [ ] **Step 1: Write failing integration checks for new routes**

Add two route checks:
- `POST /api/v1/keys/actions` (expects 200 for valid request body)
- `GET /api/v1/keys/usage/{key}` (expects 200 and JSON response)

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk bash -lc "cd go-wails && rtk go test ./tests/integration -run Phase2 -v"`
Expected: FAIL because routes are not registered yet.

- [ ] **Step 3: Register routes in server router**

Add to `go-wails/internal/server/router.go`:
- `r.Post("/api/v1/keys/actions", adminH.KeysActions)`
- `r.Get("/api/v1/keys/usage/{key}", adminH.KeyUsage)`

- [ ] **Step 4: Re-run integration test**

Run: `rtk bash -lc "cd go-wails && rtk go test ./tests/integration -run Phase2 -v"`
Expected: still FAIL until handlers are implemented.

### Task 2: Key Actions Behavior (Reset/Delete)

**Files:**
- Modify: `go-wails/internal/admin/handler_test.go`
- Modify: `go-wails/internal/admin/handler.go`
- Modify: `go-wails/internal/keypool/pool.go`
- Modify: `go-wails/internal/keypool/pool_test.go`

- [ ] **Step 1: Write failing tests for key actions**

Add tests for:
- action `reset` targeting id/key
- action `delete` removing selected key from runtime config
- invalid action returns `400`

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/admin ./internal/keypool -run 'KeysActions|Reset|Delete' -v"`
Expected: FAIL with missing methods/handlers.

- [ ] **Step 3: Implement minimal pool helpers**

Add focused methods:
- reset failures for selected IDs
- remove keys for selected IDs and expose removed keys/raw keys mapping

- [ ] **Step 4: Implement `KeysActions` handler**

Behavior:
- parse `{action, ids, keys, keyType}`
- resolve targets from pool snapshot
- support `verify|reset` (same effect) and `delete`
- return Rust-compatible summary fields: `action`, `successCount`, `failedItems`, `message`, `success`

- [ ] **Step 5: Re-run tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/admin ./internal/keypool -v"`
Expected: PASS.

### Task 3: Key Usage Endpoint

**Files:**
- Modify: `go-wails/internal/metrics/store.go`
- Modify: `go-wails/internal/metrics/store_test.go`
- Modify: `go-wails/internal/admin/handler.go`
- Modify: `go-wails/internal/admin/handler_test.go`

- [ ] **Step 1: Write failing usage tests**

Cover:
- usage grouped by `model`
- key match by key id and raw key
- period filtering (`1h`, `8h`, `24h`, `month` with fallback)

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/metrics ./internal/admin -run 'Usage|KeyUsage' -v"`
Expected: FAIL due to missing usage methods/endpoint.

- [ ] **Step 3: Implement usage aggregation in metrics store**

Add one method to filter calls by window and aggregate per model for a target key with resolver callback (`keyID -> rawKey`).

- [ ] **Step 4: Implement `KeyUsage` handler**

Add endpoint response:
```json
{
  "key": "key-1",
  "period": "24h",
  "usage": {"gemini-2.5-flash": 3}
}
```

- [ ] **Step 5: Re-run tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/metrics ./internal/admin -v"`
Expected: PASS.

### Task 4: Final Verification and Docs Sync

**Files:**
- Modify: `go-wails/API_COMPAT_GAPS.md`

- [ ] **Step 1: Run full Go-Wails tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./..."`
Expected: PASS.

- [ ] **Step 2: Update compatibility report**

Move implemented routes from missing list to implemented list and keep residual gaps unchanged.

- [ ] **Step 3: Commit**

Run:
```bash
rtk git add docs/superpowers/plans/2026-04-22-go-wails-phase3-priority1-implementation-plan.md \
  go-wails/internal/server/router.go \
  go-wails/internal/admin/handler.go \
  go-wails/internal/admin/handler_test.go \
  go-wails/internal/keypool/pool.go \
  go-wails/internal/keypool/pool_test.go \
  go-wails/internal/metrics/store.go \
  go-wails/internal/metrics/store_test.go \
  go-wails/tests/integration/phase2_contract_test.go \
  go-wails/API_COMPAT_GAPS.md
rtk git commit -m "feat(go-wails): add keys actions and key usage admin apis"
```
