# Go + Wails Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 2 migration by adding key/model pool strategies, logs, and dashboard capabilities on top of the merged `go-wails` Phase 1 baseline.

**Architecture:** Keep the existing two-entrypoint layout (`cmd/server`, `cmd/wails`) and extend the shared core with three new domains: `keypool`, `modelpool`, and `metrics`. Proxy handlers will choose model and key before upstream forwarding, then record success/failure events for logs and dashboard aggregation. Admin/Wails endpoints will expose pool status, logs querying, and dashboard overview.

**Tech Stack:** Go 1.21+, `chi`, Wails v2, Go `testing`/`httptest`, in-memory store for runtime/log metrics.

---

## Scope Check Decision

This plan only targets approved Phase 2 scope:
- `key pool` strategy behavior
- `model pool` strategy behavior
- logs and dashboard APIs/UI bindings

It does not include database persistence, scheduler/proxy-check, or additional legacy compatibility routes.

## File Structure Map (Phase 2)

- `go-wails/internal/config/config.go`
  Responsibility: parse additional Phase 2 env fields (`MAX_FAILURES`, `COOLDOWN_SECONDS`, `MODEL_POOL_STRATEGY`, `MODEL_POOL_SCOPE`).
- `go-wails/internal/config/config_test.go`
  Responsibility: validate Phase 2 config parsing.
- `go-wails/internal/keypool/pool.go`
  Responsibility: key selection strategy, failure/cooldown transitions, status snapshots.
- `go-wails/internal/keypool/pool_test.go`
  Responsibility: unit tests for round-robin/random/least-fail and cooldown recovery.
- `go-wails/internal/modelpool/pool.go`
  Responsibility: alias model mapping and strategy selection (`round_robin`, `least_used`, `per_key_cycle`).
- `go-wails/internal/modelpool/pool_test.go`
  Responsibility: unit tests for model selection and per-key-cycle behavior.
- `go-wails/internal/metrics/store.go`
  Responsibility: append/query logs, call stats, and dashboard aggregates.
- `go-wails/internal/metrics/store_test.go`
  Responsibility: unit tests for log retention and aggregate calculations.
- `go-wails/internal/runtime/manager.go`
  Responsibility: hold references to key/model/metrics stores and expose snapshots.
- `go-wails/internal/proxy/handler.go`
  Responsibility: integrate model and key selection before forwarding; report results back to stores.
- `go-wails/internal/proxy/handler_test.go`
  Responsibility: verify strategy integration and result recording.
- `go-wails/internal/admin/handler.go`
  Responsibility: add logs/dashboard/pool-status endpoints and strategy update endpoints.
- `go-wails/internal/admin/handler_test.go`
  Responsibility: API tests for new admin endpoints.
- `go-wails/internal/server/router.go`
  Responsibility: route wiring for new `/api/v1/logs`, `/api/v1/dashboard/overview`, `/api/v1/pool/*`.
- `go-wails/internal/wails/app.go`
  Responsibility: expose logs/dashboard/pool methods for UI.
- `go-wails/ui/wails/frontend/src/App.tsx`
  Responsibility: add Phase 2 panels for pool status, attention logs, and dashboard summary.
- `go-wails/tests/integration/phase2_contract_test.go`
  Responsibility: end-to-end API checks for Phase 2 routes.
- `go-wails/scripts/smoke-phase2.sh`
  Responsibility: one-command Phase 2 smoke run (models/chat + logs + dashboard).
- `go-wails/README.md`
  Responsibility: add Phase 2 runbook and verification steps.

### Task 1: Extend Config Contract for Phase 2 Fields

**Files:**
- Modify: `go-wails/internal/config/config.go`
- Modify: `go-wails/internal/config/config_test.go`

- [ ] **Step 1: Write failing tests for new env fields**

```go
func TestLoadFromEnv_ParsesPhase2Fields(t *testing.T) {
	env := map[string]string{
		"MAX_FAILURES":        "5",
		"COOLDOWN_SECONDS":    "90",
		"MODEL_POOL_STRATEGY": "per_key_cycle",
		"MODEL_POOL_SCOPE":    "global",
	}
	cfg, err := LoadFromEnv(env)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.MaxFailures != 5 || cfg.CooldownSeconds != 90 {
		t.Fatalf("numeric fields mismatch: %#v", cfg)
	}
	if cfg.ModelPoolStrategy != "per_key_cycle" || cfg.ModelPoolScope != "global" {
		t.Fatalf("strategy fields mismatch: %#v", cfg)
	}
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/config -run TestLoadFromEnv_ParsesPhase2Fields -v"`  
Expected: FAIL with missing struct fields.

- [ ] **Step 3: Implement config fields and parsing**

```go
type AppConfig struct {
	AuthToken         string
	AllowedTokens     []string
	APIKeys           []string
	BindHost          string
	PortStart         int
	PortEnd           int
	PoolStrategy      string
	ModelPools        map[string][]string
	MaxFailures       int
	CooldownSeconds   int
	ModelPoolStrategy string
	ModelPoolScope    string
}
```

```go
maxFailures, err := atoiDefault(env["MAX_FAILURES"], 3)
if err != nil {
	return AppConfig{}, fmt.Errorf("invalid MAX_FAILURES: %w", err)
}
cooldownSecs, err := atoiDefault(env["COOLDOWN_SECONDS"], 60)
if err != nil {
	return AppConfig{}, fmt.Errorf("invalid COOLDOWN_SECONDS: %w", err)
}
```

```go
MaxFailures:       maxFailures,
CooldownSeconds:   cooldownSecs,
ModelPoolStrategy: valueOr(env["MODEL_POOL_STRATEGY"], "round_robin"),
ModelPoolScope:    valueOr(env["MODEL_POOL_SCOPE"], "global"),
```

- [ ] **Step 4: Run config tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/config -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/config/config.go go-wails/internal/config/config_test.go
rtk git commit -m "feat(go-wails): add phase2 config fields"
```

### Task 2: Implement Key Pool Domain

**Files:**
- Create: `go-wails/internal/keypool/pool.go`
- Create: `go-wails/internal/keypool/pool_test.go`

- [ ] **Step 1: Write failing key pool tests**

```go
func TestKeyPool_RoundRobinRotation(t *testing.T) {
	p := NewPool([]string{"k1", "k2"}, 3, 60, "round_robin")
	first, ok := p.Next("user-a", time.Now())
	if !ok {
		t.Fatal("expected first key")
	}
	second, ok := p.Next("user-a", time.Now().Add(time.Second))
	if !ok {
		t.Fatal("expected second key")
	}
	if first.RawKey == second.RawKey {
		t.Fatalf("expected rotation, got same key %s", first.RawKey)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/keypool -v"`  
Expected: FAIL with undefined pool types/functions.

- [ ] **Step 3: Implement key pool**

```go
type KeyState struct {
	ID            string
	RawKey        string
	FailureCount  int
	CooldownUntil time.Time
	LastUsedAt    time.Time
}

type Pool struct {
	mu           sync.Mutex
	items        []KeyState
	next         int
	maxFailures  int
	cooldownSecs int
	strategy     string
}
```

```go
func (p *Pool) Next(token string, now time.Time) (KeyState, bool) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for i := range p.items {
		if !p.items[i].CooldownUntil.IsZero() && now.After(p.items[i].CooldownUntil) {
			p.items[i].FailureCount = 0
			p.items[i].CooldownUntil = time.Time{}
		}
	}
	idx := p.selectIndex(now)
	if idx < 0 {
		return KeyState{}, false
	}
	p.items[idx].LastUsedAt = now
	return p.items[idx], true
}
```

```go
func (p *Pool) MarkFailure(rawKey string, now time.Time) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for i := range p.items {
		if p.items[i].RawKey == rawKey {
			p.items[i].FailureCount++
			if p.items[i].FailureCount >= p.maxFailures {
				p.items[i].CooldownUntil = now.Add(time.Duration(p.cooldownSecs) * time.Second)
			}
			return
		}
	}
}
func (p *Pool) MarkSuccess(rawKey string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for i := range p.items {
		if p.items[i].RawKey == rawKey {
			p.items[i].FailureCount = 0
			p.items[i].CooldownUntil = time.Time{}
			return
		}
	}
}
func (p *Pool) Snapshot() []KeyState {
	p.mu.Lock()
	defer p.mu.Unlock()
	out := make([]KeyState, len(p.items))
	copy(out, p.items)
	return out
}
```

- [ ] **Step 4: Run key pool tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/keypool -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/keypool/pool.go go-wails/internal/keypool/pool_test.go
rtk git commit -m "feat(go-wails): implement phase2 key pool strategies"
```

### Task 3: Implement Model Pool Domain

**Files:**
- Create: `go-wails/internal/modelpool/pool.go`
- Create: `go-wails/internal/modelpool/pool_test.go`

- [ ] **Step 1: Write failing model pool tests**

```go
func TestModelPool_PerKeyCycle(t *testing.T) {
	p := NewPool(map[string][]string{"fast": []string{"m1", "m2"}}, "per_key_cycle")
	m1 := p.Resolve("fast", "key-a")
	m2 := p.Resolve("fast", "key-a")
	if m1 != m2 {
		t.Fatalf("same key in cycle should keep model, got %s/%s", m1, m2)
	}
	m3 := p.Resolve("fast", "key-b")
	if m3 == "" {
		t.Fatal("expected non-empty model")
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/modelpool -v"`  
Expected: FAIL with undefined model pool symbols.

- [ ] **Step 3: Implement model pool**

```go
type Pool struct {
	mu        sync.Mutex
	aliases   map[string][]string
	strategy  string
	cursor    map[string]int
	usage     map[string]map[string]uint64
	lastByKey map[string]map[string]string
}
```

```go
func (p *Pool) Resolve(requestedModel, keyID string) string {
	p.mu.Lock()
	defer p.mu.Unlock()
	targets, ok := p.aliases[requestedModel]
	if !ok || len(targets) == 0 {
		return requestedModel
	}
	if p.strategy == "per_key_cycle" {
		if v, ok := p.lastByKey[requestedModel][keyID]; ok && v != "" {
			return v
		}
	}
	if p.strategy == "least_used" {
		best := targets[0]
		bestCount := p.usage[requestedModel][best]
		for _, candidate := range targets[1:] {
			if p.usage[requestedModel][candidate] < bestCount {
				best = candidate
				bestCount = p.usage[requestedModel][candidate]
			}
		}
		p.lastByKey[requestedModel][keyID] = best
		return best
	}
	idx := p.cursor[requestedModel] % len(targets)
	selected := targets[idx]
	p.cursor[requestedModel] = (idx + 1) % len(targets)
	p.lastByKey[requestedModel][keyID] = selected
	return selected
}
```

```go
func (p *Pool) MarkUsed(alias, actual string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if _, ok := p.usage[alias]; !ok {
		p.usage[alias] = map[string]uint64{}
	}
	p.usage[alias][actual]++
}
```

- [ ] **Step 4: Run model pool tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/modelpool -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/modelpool/pool.go go-wails/internal/modelpool/pool_test.go
rtk git commit -m "feat(go-wails): implement phase2 model pool strategies"
```

### Task 4: Implement Logs and Dashboard Metrics Store

**Files:**
- Create: `go-wails/internal/metrics/store.go`
- Create: `go-wails/internal/metrics/store_test.go`

- [ ] **Step 1: Write failing metrics tests**

```go
func TestStore_RecordAndOverview(t *testing.T) {
	s := NewStore(2000)
	now := time.Now()
	s.RecordCall(CallRecord{At: now, KeyID: "key-1", Model: "m1", StatusCode: 200})
	s.RecordCall(CallRecord{At: now, KeyID: "key-1", Model: "m1", StatusCode: 429})
	overview := s.DashboardOverview(now)
	if overview.Calls24h.Total != 2 || overview.Calls24h.Failure != 1 {
		t.Fatalf("unexpected overview: %#v", overview)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/metrics -v"`  
Expected: FAIL with undefined store symbols.

- [ ] **Step 3: Implement in-memory metrics store**

```go
type CallRecord struct {
	At         time.Time
	KeyID      string
	MaskedKey  string
	Model      string
	StatusCode int
	Detail     string
}

type Store struct {
	mu      sync.RWMutex
	calls   []CallRecord
	maxLogs int
}
```

```go
func (s *Store) RecordCall(rec CallRecord) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.calls = append(s.calls, rec)
	if len(s.calls) > s.maxLogs {
		s.calls = s.calls[len(s.calls)-s.maxLogs:]
	}
}
func (s *Store) Logs(limit, offset int) []CallRecord {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if offset >= len(s.calls) {
		return []CallRecord{}
	}
	end := offset + limit
	if end > len(s.calls) {
		end = len(s.calls)
	}
	out := make([]CallRecord, end-offset)
	copy(out, s.calls[offset:end])
	return out
}
func (s *Store) DashboardOverview(now time.Time) Overview {
	s.mu.RLock()
	defer s.mu.RUnlock()
	var total, success, failure uint64
	cut := now.Add(-24 * time.Hour)
	for _, rec := range s.calls {
		if rec.At.Before(cut) {
			continue
		}
		total++
		if rec.StatusCode >= 200 && rec.StatusCode < 300 {
			success++
		} else {
			failure++
		}
	}
	return Overview{Calls24h: CallsSummary{Total: total, Success: success, Failure: failure}}
}
```

- [ ] **Step 4: Run metrics tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/metrics -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/metrics/store.go go-wails/internal/metrics/store_test.go
rtk git commit -m "feat(go-wails): add phase2 logs and dashboard metrics store"
```

### Task 5: Integrate Key/Model/Metrics into Runtime and Proxy

**Files:**
- Modify: `go-wails/internal/runtime/manager.go`
- Modify: `go-wails/internal/proxy/handler.go`
- Modify: `go-wails/internal/proxy/handler_test.go`

- [ ] **Step 1: Write failing integration tests in proxy handler**

```go
func TestChatCompletions_UsesAliasAndRecordsMetrics(t *testing.T) {
	cfg := config.AppConfig{
		AllowedTokens:     []string{"sk-user-1"},
		APIKeys:           []string{"k1"},
		ModelPools:        map[string][]string{"fast": []string{"m1", "m2"}},
		ModelPoolStrategy: "round_robin",
	}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(cfg, &FakeClient{}, mgr)
	req := httptest.NewRequest(http.MethodPost, "/v1/chat/completions", strings.NewReader(`{"model":"fast","messages":[{"role":"user","content":"hi"}]}`))
	req.Header.Set("Authorization", "Bearer sk-user-1")
	w := httptest.NewRecorder()
	h.ChatCompletions(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	if len(mgr.Metrics().Logs(10, 0)) != 1 {
		t.Fatalf("expected one log record")
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/proxy -v"`  
Expected: FAIL with missing runtime integrations.

- [ ] **Step 3: Implement runtime dependencies and proxy flow**

```go
type Manager struct {
	keyPool   *keypool.Pool
	modelPool *modelpool.Pool
	metrics   *metrics.Store
}
```

```go
func (h *Handler) ChatCompletions(w http.ResponseWriter, r *http.Request) {
	token := auth.ExtractProxyToken(r)
	if !auth.IsAllowedProxyToken(token, h.cfg.AllowedTokens) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	now := time.Now()
	key, ok := h.mgr.KeyPool().Next(token, now)
	if !ok {
		http.Error(w, "no key available", http.StatusServiceUnavailable)
		return
	}
	body, _ := io.ReadAll(r.Body)
	reqModel := extractModel(body)
	actualModel := h.mgr.ModelPool().Resolve(reqModel, key.ID)
	body = replaceModel(body, actualModel)
	status, resp, err := h.client.Chat(key.RawKey, body, r.URL.RawQuery)
	if err != nil || status >= 400 {
		h.mgr.KeyPool().MarkFailure(key.RawKey, now)
	} else {
		h.mgr.KeyPool().MarkSuccess(key.RawKey)
	}
	h.mgr.ModelPool().MarkUsed(reqModel, actualModel)
	h.mgr.Metrics().RecordCall(metrics.CallRecord{At: now, KeyID: key.ID, Model: actualModel, StatusCode: status})
	w.WriteHeader(status)
	_, _ = w.Write(resp)
}
```

- [ ] **Step 4: Run proxy + runtime tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/runtime ./internal/proxy -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/runtime/manager.go go-wails/internal/proxy/handler.go go-wails/internal/proxy/handler_test.go
rtk git commit -m "feat(go-wails): integrate phase2 key model metrics in proxy path"
```

### Task 6: Add Admin and Wails APIs for Pool/Logs/Dashboard

**Files:**
- Modify: `go-wails/internal/admin/handler.go`
- Modify: `go-wails/internal/admin/handler_test.go`
- Modify: `go-wails/internal/server/router.go`
- Modify: `go-wails/internal/wails/app.go`
- Modify: `go-wails/ui/wails/frontend/src/App.tsx`

- [ ] **Step 1: Write failing admin endpoint tests**

```go
func TestDashboardOverviewRoute(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin", APIKeys: []string{"k1"}}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(mgr, ".env.test")
	req := httptest.NewRequest(http.MethodGet, "/api/v1/dashboard/overview", nil)
	w := httptest.NewRecorder()
	h.DashboardOverview(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}

func TestLogsRoute(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin", APIKeys: []string{"k1"}}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(mgr, ".env.test")
	req := httptest.NewRequest(http.MethodGet, "/api/v1/logs?limit=20&offset=0", nil)
	w := httptest.NewRecorder()
	h.Logs(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/admin ./internal/server -v"`  
Expected: FAIL with missing handlers/routes.

- [ ] **Step 3: Implement admin + Wails methods**

```go
func (h *Handler) DashboardOverview(w http.ResponseWriter, r *http.Request) {
	now := time.Now()
	_ = json.NewEncoder(w).Encode(h.mgr.Metrics().DashboardOverview(now))
}

func (h *Handler) Logs(w http.ResponseWriter, r *http.Request) {
	limit := parseIntQuery(r, "limit", 20)
	offset := parseIntQuery(r, "offset", 0)
	_ = json.NewEncoder(w).Encode(map[string]any{"logs": h.mgr.Metrics().Logs(limit, offset)})
}
```

```go
r.Get("/api/v1/logs", adminH.Logs)
r.Get("/api/v1/dashboard/overview", adminH.DashboardOverview)
r.Get("/api/v1/pool/status", adminH.PoolStatus)
r.Put("/api/v1/pool/strategy", adminH.UpdatePoolStrategy)
```

```go
func (a *App) GetDashboardOverview() any { return a.mgr.Metrics().DashboardOverview(time.Now()) }
func (a *App) GetLogs(limit, offset int) any { return a.mgr.Metrics().Logs(limit, offset) }
func (a *App) GetPoolStatus() any { return a.mgr.KeyPool().Snapshot() }
```

- [ ] **Step 4: Run admin/server tests and frontend build**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/admin ./internal/server -v"`  
Expected: PASS.

Run: `rtk bash -lc "cd go-wails/ui/wails/frontend && rtk npm run build"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/admin/handler.go go-wails/internal/admin/handler_test.go go-wails/internal/server/router.go go-wails/internal/wails/app.go go-wails/ui/wails/frontend/src/App.tsx
rtk git commit -m "feat(go-wails): expose phase2 pool logs dashboard APIs and UI"
```

### Task 7: Add Phase 2 Integration Tests, Smoke Script, and Docs

**Files:**
- Create: `go-wails/tests/integration/phase2_contract_test.go`
- Create: `go-wails/scripts/smoke-phase2.sh`
- Modify: `go-wails/README.md`

- [ ] **Step 1: Write failing phase2 integration test**

```go
func TestPhase2_DashboardAndLogsRoutes(t *testing.T) {
	cfg := config.AppConfig{
		AuthToken:     "sk-admin-1",
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"k1"},
		BindHost:      "127.0.0.1",
		PortStart:     18080,
	}
	mgr := runtime.NewManager(cfg)
	adminH := admin.NewHandler(mgr, ".env.test")
	proxyH := proxy.NewHandler(cfg, &proxy.NoopClient{}, mgr)
	router := server.NewRouter(adminH, proxyH)
	ts := httptest.NewServer(router)
	defer ts.Close()
	resp1, err := http.Get(ts.URL + "/api/v1/dashboard/overview")
	if err != nil || resp1.StatusCode != http.StatusOK {
		t.Fatalf("dashboard route failed, status=%v err=%v", resp1.StatusCode, err)
	}
	resp2, err := http.Get(ts.URL + "/api/v1/logs?limit=20&offset=0")
	if err != nil || resp2.StatusCode != http.StatusOK {
		t.Fatalf("logs route failed, status=%v err=%v", resp2.StatusCode, err)
	}
}
```

- [ ] **Step 2: Run integration test to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./tests/integration -run TestPhase2_DashboardAndLogsRoutes -v"`  
Expected: FAIL before route/handler completion.

- [ ] **Step 3: Add smoke script and docs**

```bash
#!/usr/bin/env bash
set -euo pipefail
BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
PROXY_TOKEN="${PROXY_TOKEN:-sk-user-demo}"
rtk curl -sS "${BASE_URL}/v1/models" -H "Authorization: Bearer ${PROXY_TOKEN}" >/tmp/go-wails-phase2-models.json
rtk curl -sS "${BASE_URL}/api/v1/dashboard/overview" >/tmp/go-wails-phase2-dashboard.json
rtk curl -sS "${BASE_URL}/api/v1/logs?limit=20&offset=0" >/tmp/go-wails-phase2-logs.json
echo "[smoke-phase2] done"
```

````markdown
## Phase 2 Verify

```bash
cd go-wails
rtk go test ./...
rtk bash scripts/smoke-phase2.sh
```
````

- [ ] **Step 4: Run full verification**

Run: `rtk bash -lc "cd go-wails && rtk go test ./..."`  
Expected: PASS.

Run: `rtk bash -lc "cd go-wails && rtk bash scripts/smoke-phase2.sh"`  
Expected: prints `[smoke-phase2] done`.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/tests/integration/phase2_contract_test.go go-wails/scripts/smoke-phase2.sh go-wails/README.md
rtk git commit -m "test(go-wails): add phase2 integration and smoke verification"
```

## Self-Review Checklist (Completed)

1. **Spec coverage:**  
   - Key pool, model pool, logs, dashboard each map to dedicated tasks.
   - Runtime/proxy/admin/UI integration and verification are explicitly covered.
2. **Placeholder scan:**  
   - No `TBD`, `TODO`, or deferred implementation placeholders remain.
3. **Type consistency:**  
   - `config.AppConfig`, `runtime.Manager`, `keypool.Pool`, `modelpool.Pool`, and `metrics.Store` are used consistently across tasks.

## Execution Handoff

After this plan is approved, execute with:
- `superpowers:subagent-driven-development` (recommended), or
- `superpowers:executing-plans` (inline alternative).
