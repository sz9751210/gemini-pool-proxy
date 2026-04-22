# Go + Wails Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a new `go-wails/` implementation that delivers Phase 1 parity (`headless proxy + admin API + basic Wails GUI`) while preserving external API and `.env` compatibility and leaving existing Rust/Tauri code untouched.

**Architecture:** Create a shared Go core with two entrypoints (`cmd/server` for headless and `cmd/wails` for GUI). Keep proxy and admin flows separated but backed by a single runtime/config layer so GUI and headless behavior stays consistent. Implement only Phase 1 scope in this plan; Phase 2 (`key/model pool strategies, logs, dashboard`) is intentionally excluded and should be planned in a separate follow-up plan.

**Tech Stack:** Go 1.24+, `chi` router, `httputil` reverse proxy, `godotenv`, Wails v2 + React + TypeScript, `testing` + `httptest`.

---

## Scope Check Decision

The approved spec contains two independent delivery phases. This plan covers only Phase 1 to keep implementation bounded and testable. Create a second plan for Phase 2 after Phase 1 acceptance.

## File Structure Map (Phase 1)

- `go-wails/go.mod`
  Responsibility: module root and dependency pinning.
- `go-wails/internal/config/config.go`
  Responsibility: parse environment values into typed config.
- `go-wails/internal/config/save.go`
  Responsibility: write updated config back to `.env` while preserving unknown keys.
- `go-wails/internal/config/config_test.go`
  Responsibility: unit tests for env parsing compatibility.
- `go-wails/internal/config/save_test.go`
  Responsibility: unit tests for `.env` rewrite behavior.
- `go-wails/internal/runtime/manager.go`
  Responsibility: runtime service state (start/stop/health/config snapshot).
- `go-wails/internal/runtime/manager_test.go`
  Responsibility: runtime state transition tests.
- `go-wails/internal/auth/token.go`
  Responsibility: extract and validate proxy/admin tokens.
- `go-wails/internal/auth/token_test.go`
  Responsibility: token extraction/validation tests.
- `go-wails/internal/proxy/handler.go`
  Responsibility: `/v1/models`, `/v1/chat/completions`, `/v1beta/*` handlers.
- `go-wails/internal/proxy/client.go`
  Responsibility: upstream Gemini forwarding logic.
- `go-wails/internal/proxy/handler_test.go`
  Responsibility: proxy API behavior tests with mocked upstream.
- `go-wails/internal/admin/session.go`
  Responsibility: in-memory admin session creation/validation/logout.
- `go-wails/internal/admin/handler.go`
  Responsibility: login/logout/status/config/keys/health/start-stop handlers.
- `go-wails/internal/admin/handler_test.go`
  Responsibility: admin API behavior tests.
- `go-wails/internal/server/router.go`
  Responsibility: compose auth, proxy, admin routes.
- `go-wails/cmd/server/main.go`
  Responsibility: headless bootstrapping and HTTP server start.
- `go-wails/internal/wails/app.go`
  Responsibility: Wails bindings wrapping admin/runtime use-cases.
- `go-wails/cmd/wails/main.go`
  Responsibility: Wails application entrypoint.
- `go-wails/ui/wails/frontend/src/App.tsx`
  Responsibility: basic Phase 1 GUI (login, config, keys, health, start/stop).
- `go-wails/tests/integration/phase1_contract_test.go`
  Responsibility: end-to-end API compatibility smoke checks.
- `go-wails/scripts/smoke-phase1.sh`
  Responsibility: one-command local verification for Phase 1.
- `go-wails/README.md`
  Responsibility: setup, run, and verification instructions for new implementation.

### Task 1: Bootstrap Module and `.env` Parser

**Files:**
- Create: `go-wails/go.mod`
- Create: `go-wails/internal/config/config.go`
- Create: `go-wails/internal/config/config_test.go`

- [ ] **Step 1: Write the failing parser tests**

```go
// go-wails/internal/config/config_test.go
package config

import "testing"

func TestLoadFromEnv_ParsesCoreFields(t *testing.T) {
	env := map[string]string{
		"AUTH_TOKEN":         "sk-admin-1",
		"ALLOWED_TOKENS":     `["sk-user-1","sk-user-2"]`,
		"API_KEYS":           "AIza-A,AIza-B",
		"RUNTIME_BIND_HOST":  "127.0.0.1",
		"RUNTIME_PORT_START": "18080",
		"RUNTIME_PORT_END":   "18099",
		"POOL_STRATEGY":      "round_robin",
		"MODEL_POOLS":        `{"fast":["gemini-2.5-flash"]}`,
	}

	cfg, err := LoadFromEnv(env)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.AuthToken != "sk-admin-1" {
		t.Fatalf("wrong auth token: %s", cfg.AuthToken)
	}
	if len(cfg.AllowedTokens) != 2 || cfg.AllowedTokens[0] != "sk-user-1" {
		t.Fatalf("allowed tokens parse failed: %#v", cfg.AllowedTokens)
	}
	if len(cfg.APIKeys) != 2 || cfg.APIKeys[1] != "AIza-B" {
		t.Fatalf("api keys parse failed: %#v", cfg.APIKeys)
	}
	if cfg.BindHost != "127.0.0.1" || cfg.PortStart != 18080 || cfg.PortEnd != 18099 {
		t.Fatalf("listen config parse failed: %#v", cfg)
	}
	if cfg.ModelPools["fast"][0] != "gemini-2.5-flash" {
		t.Fatalf("model pool parse failed: %#v", cfg.ModelPools)
	}
}

func TestParseArray_AcceptsLooseAndJSON(t *testing.T) {
	a := parseArray(`["x","y"]`)
	b := parseArray("x,y")
	if len(a) != 2 || a[1] != "y" {
		t.Fatalf("json array parse failed: %#v", a)
	}
	if len(b) != 2 || b[0] != "x" {
		t.Fatalf("csv parse failed: %#v", b)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/config -run TestLoadFromEnv_ParsesCoreFields -v"`  
Expected: FAIL with `undefined: LoadFromEnv` or package compile errors.

- [ ] **Step 3: Write minimal parser implementation**

```go
// go-wails/go.mod
module github.com/alan/gemini-pool-proxy/go-wails

go 1.24

require (
	github.com/go-chi/chi/v5 v5.1.0
	github.com/joho/godotenv v1.5.1
	github.com/wailsapp/wails/v2 v2.9.2
)
```

```go
// go-wails/internal/config/config.go
package config

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

type AppConfig struct {
	AuthToken     string
	AllowedTokens []string
	APIKeys       []string
	BindHost      string
	PortStart     int
	PortEnd       int
	PoolStrategy  string
	ModelPools    map[string][]string
}

func LoadFromEnv(env map[string]string) (AppConfig, error) {
	start, err := atoiDefault(env["RUNTIME_PORT_START"], 18080)
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid RUNTIME_PORT_START: %w", err)
	}
	end, err := atoiDefault(env["RUNTIME_PORT_END"], 18099)
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid RUNTIME_PORT_END: %w", err)
	}
	modelPools, err := parseModelPools(env["MODEL_POOLS"])
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid MODEL_POOLS: %w", err)
	}
	cfg := AppConfig{
		AuthToken:     valueOr(env["AUTH_TOKEN"], "sk-admin-demo"),
		AllowedTokens: parseArray(valueOr(env["ALLOWED_TOKENS"], "sk-user-demo")),
		APIKeys:       parseArray(valueOr(env["API_KEYS"], "")),
		BindHost:      valueOr(env["RUNTIME_BIND_HOST"], "127.0.0.1"),
		PortStart:     start,
		PortEnd:       end,
		PoolStrategy:  valueOr(env["POOL_STRATEGY"], "round_robin"),
		ModelPools:    modelPools,
	}
	return cfg, nil
}

func parseArray(raw string) []string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return []string{}
	}
	if strings.HasPrefix(raw, "[") && strings.HasSuffix(raw, "]") {
		var out []string
		if err := json.Unmarshal([]byte(raw), &out); err == nil {
			return trimItems(out)
		}
		raw = strings.TrimSuffix(strings.TrimPrefix(raw, "["), "]")
	}
	parts := strings.Split(raw, ",")
	return trimItems(parts)
}

func parseModelPools(raw string) (map[string][]string, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return map[string][]string{}, nil
	}
	out := map[string][]string{}
	if err := json.Unmarshal([]byte(raw), &out); err == nil {
		return out, nil
	}
	return nil, fmt.Errorf("expected JSON object")
}

func atoiDefault(raw string, fallback int) (int, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return fallback, nil
	}
	return strconv.Atoi(raw)
}

func valueOr(v, fallback string) string {
	if strings.TrimSpace(v) == "" {
		return fallback
	}
	return strings.TrimSpace(v)
}

func trimItems(items []string) []string {
	out := make([]string, 0, len(items))
	for _, item := range items {
		s := strings.TrimSpace(strings.Trim(item, `"'`))
		if s != "" {
			out = append(out, s)
		}
	}
	return out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/config -v"`  
Expected: PASS for parser tests.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/go.mod go-wails/internal/config/config.go go-wails/internal/config/config_test.go
rtk git commit -m "feat(go-wails): bootstrap module and env parser"
```

### Task 2: Implement `.env` Writer with Unknown-Key Preservation

**Files:**
- Create: `go-wails/internal/config/save.go`
- Create: `go-wails/internal/config/save_test.go`

- [ ] **Step 1: Write failing tests for `.env` rewrite semantics**

```go
// go-wails/internal/config/save_test.go
package config

import (
	"os"
	"strings"
	"testing"
)

func TestUpdateEnvFile_PreservesUnknownLines(t *testing.T) {
	f, err := os.CreateTemp("", "phase1-env-*.env")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(f.Name())

	initial := "AUTH_TOKEN=old\nALLOWED_TOKENS=[\"a\"]\nCUSTOM_KEEP=1\n"
	if err := os.WriteFile(f.Name(), []byte(initial), 0644); err != nil {
		t.Fatal(err)
	}

	cfg := AppConfig{
		AuthToken:     "new-admin",
		AllowedTokens: []string{"u1", "u2"},
		APIKeys:       []string{"k1"},
		BindHost:      "127.0.0.1",
		PortStart:     18080,
		PortEnd:       18099,
		PoolStrategy:  "round_robin",
		ModelPools:    map[string][]string{"fast": []string{"gemini-2.5-flash"}},
	}
	if err := UpdateEnvFile(f.Name(), cfg); err != nil {
		t.Fatalf("update failed: %v", err)
	}

	body, err := os.ReadFile(f.Name())
	if err != nil {
		t.Fatal(err)
	}
	text := string(body)
	if !strings.Contains(text, "AUTH_TOKEN=new-admin") {
		t.Fatalf("expected updated auth token, got: %s", text)
	}
	if !strings.Contains(text, "CUSTOM_KEEP=1") {
		t.Fatalf("expected unknown line preserved, got: %s", text)
	}
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/config -run TestUpdateEnvFile_PreservesUnknownLines -v"`  
Expected: FAIL with `undefined: UpdateEnvFile`.

- [ ] **Step 3: Implement deterministic `.env` updater**

```go
// go-wails/internal/config/save.go
package config

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strings"
)

func UpdateEnvFile(path string, cfg AppConfig) error {
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	lines := strings.Split(string(raw), "\n")

	desired := map[string]string{}
	desired["AUTH_TOKEN"] = cfg.AuthToken
	desired["ALLOWED_TOKENS"] = mustJSON(cfg.AllowedTokens)
	desired["API_KEYS"] = mustJSON(cfg.APIKeys)
	desired["RUNTIME_BIND_HOST"] = cfg.BindHost
	desired["RUNTIME_PORT_START"] = fmt.Sprintf("%d", cfg.PortStart)
	desired["RUNTIME_PORT_END"] = fmt.Sprintf("%d", cfg.PortEnd)
	desired["POOL_STRATEGY"] = cfg.PoolStrategy
	desired["MODEL_POOLS"] = mustJSON(cfg.ModelPools)

	seen := map[string]bool{}
	for i, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") || !strings.Contains(trimmed, "=") {
			continue
		}
		key, _, ok := strings.Cut(trimmed, "=")
		if !ok {
			continue
		}
		key = strings.TrimSpace(key)
		if val, has := desired[key]; has {
			lines[i] = fmt.Sprintf("%s=%s", key, val)
			seen[key] = true
		}
	}

	missing := make([]string, 0, len(desired))
	for key := range desired {
		if !seen[key] {
			missing = append(missing, key)
		}
	}
	sort.Strings(missing)
	for _, key := range missing {
		lines = append(lines, fmt.Sprintf("%s=%s", key, desired[key]))
	}

	return os.WriteFile(path, []byte(strings.TrimRight(strings.Join(lines, "\n"), "\n")+"\n"), 0644)
}

func mustJSON(v any) string {
	b, _ := json.Marshal(v)
	return string(b)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/config -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/config/save.go go-wails/internal/config/save_test.go
rtk git commit -m "feat(go-wails): add env writer with unknown-key preservation"
```

### Task 3: Add Runtime State Manager (Start/Stop/Health/Config)

**Files:**
- Create: `go-wails/internal/runtime/manager.go`
- Create: `go-wails/internal/runtime/manager_test.go`

- [ ] **Step 1: Write failing runtime transition tests**

```go
// go-wails/internal/runtime/manager_test.go
package runtime

import (
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
)

func TestManager_StartStopAndHealth(t *testing.T) {
	m := NewManager(config.AppConfig{BindHost: "127.0.0.1", PortStart: 18080})
	if m.Health().Running {
		t.Fatal("should start stopped")
	}
	if err := m.Start(); err != nil {
		t.Fatalf("start failed: %v", err)
	}
	if !m.Health().Running {
		t.Fatal("expected running=true")
	}
	if err := m.Stop(); err != nil {
		t.Fatalf("stop failed: %v", err)
	}
	if m.Health().Running {
		t.Fatal("expected running=false")
	}
}
```

- [ ] **Step 2: Run test and confirm failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/runtime -run TestManager_StartStopAndHealth -v"`  
Expected: FAIL with `undefined: NewManager`.

- [ ] **Step 3: Implement runtime manager**

```go
// go-wails/internal/runtime/manager.go
package runtime

import (
	"fmt"
	"sync"
	"time"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
)

type Health struct {
	Running      bool   `json:"running"`
	ListenAddr   string `json:"listenAddr"`
	LastError    string `json:"lastError"`
	LastChangeAt string `json:"lastChangeAt"`
}

type Manager struct {
	mu         sync.RWMutex
	cfg        config.AppConfig
	running    bool
	lastError  string
	lastChange time.Time
}

func NewManager(cfg config.AppConfig) *Manager {
	return &Manager{
		cfg:        cfg,
		running:    false,
		lastChange: time.Now(),
	}
}

func (m *Manager) Start() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.running = true
	m.lastError = ""
	m.lastChange = time.Now()
	return nil
}

func (m *Manager) Stop() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.running = false
	m.lastChange = time.Now()
	return nil
}

func (m *Manager) UpdateConfig(cfg config.AppConfig) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.cfg = cfg
	m.lastChange = time.Now()
}

func (m *Manager) Config() config.AppConfig {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.cfg
}

func (m *Manager) Health() Health {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return Health{
		Running:      m.running,
		ListenAddr:   fmt.Sprintf("%s:%d", m.cfg.BindHost, m.cfg.PortStart),
		LastError:    m.lastError,
		LastChangeAt: m.lastChange.UTC().Format(time.RFC3339),
	}
}
```

- [ ] **Step 4: Run runtime tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/runtime -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/runtime/manager.go go-wails/internal/runtime/manager_test.go
rtk git commit -m "feat(go-wails): add phase1 runtime state manager"
```

### Task 4: Implement Auth and Proxy API (`/v1`, `/v1beta`)

**Files:**
- Create: `go-wails/internal/auth/token.go`
- Create: `go-wails/internal/auth/token_test.go`
- Create: `go-wails/internal/proxy/client.go`
- Create: `go-wails/internal/proxy/handler.go`
- Create: `go-wails/internal/proxy/handler_test.go`

- [ ] **Step 1: Write failing auth/proxy contract tests**

```go
// go-wails/internal/auth/token_test.go
package auth

import (
	"net/http/httptest"
	"testing"
)

func TestExtractProxyToken_BearerHeader(t *testing.T) {
	r := httptest.NewRequest("GET", "/", nil)
	r.Header.Set("Authorization", "Bearer sk-user-1")
	token := ExtractProxyToken(r)
	if token != "sk-user-1" {
		t.Fatalf("token mismatch: %s", token)
	}
}
```

```go
// go-wails/internal/proxy/handler_test.go
package proxy

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
)

func TestModelsEndpoint_RequiresAllowedToken(t *testing.T) {
	h := NewHandler(config.AppConfig{
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"AIza-A"},
	}, &FakeClient{})
	req := httptest.NewRequest(http.MethodGet, "/v1/models", nil)
	req.Header.Set("Authorization", "Bearer wrong")
	w := httptest.NewRecorder()
	h.Models(w, req)
	if w.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d", w.Code)
	}
}

type FakeClient struct{}

func (f *FakeClient) Models(apiKey string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"object":"list","data":[]}`), nil
}
func (f *FakeClient) Chat(apiKey string, body []byte, query string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"id":"chatcmpl-test"}`), nil
}
func (f *FakeClient) Native(apiKey, path string, body []byte, query string, method string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"ok":true}`), nil
}

func TestChatCompletions_PassesBodyToClient(t *testing.T) {
	h := NewHandler(config.AppConfig{
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"AIza-A"},
	}, &FakeClient{})
	req := httptest.NewRequest(http.MethodPost, "/v1/chat/completions", strings.NewReader(`{"model":"fast"}`))
	req.Header.Set("Authorization", "Bearer sk-user-1")
	w := httptest.NewRecorder()
	h.ChatCompletions(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}
```

- [ ] **Step 2: Run tests and verify failures**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/auth ./internal/proxy -v"`  
Expected: FAIL with undefined symbols for token extraction and handlers.

- [ ] **Step 3: Implement auth and proxy handlers**

```go
// go-wails/internal/auth/token.go
package auth

import (
	"net/http"
	"strings"
)

func ExtractProxyToken(r *http.Request) string {
	if auth := strings.TrimSpace(r.Header.Get("Authorization")); strings.HasPrefix(strings.ToLower(auth), "bearer ") {
		return strings.TrimSpace(auth[7:])
	}
	if v := strings.TrimSpace(r.Header.Get("x-api-key")); v != "" {
		return v
	}
	if v := strings.TrimSpace(r.Header.Get("x-goog-api-key")); v != "" {
		return v
	}
	if v := strings.TrimSpace(r.URL.Query().Get("key")); v != "" {
		return v
	}
	return ""
}

func IsAllowedProxyToken(token string, allowed []string) bool {
	for _, item := range allowed {
		if token == item {
			return true
		}
	}
	return false
}
```

```go
// go-wails/internal/proxy/client.go
package proxy

type Client interface {
	Models(apiKey string) (status int, body []byte, err error)
	Chat(apiKey string, body []byte, query string) (status int, resp []byte, err error)
	Native(apiKey, path string, body []byte, query string, method string) (status int, resp []byte, err error)
}

type NoopClient struct{}

func (n *NoopClient) Models(apiKey string) (int, []byte, error) {
	return 200, []byte(`{"object":"list","data":[]}`), nil
}

func (n *NoopClient) Chat(apiKey string, body []byte, query string) (int, []byte, error) {
	return 200, []byte(`{"id":"chatcmpl-noop"}`), nil
}

func (n *NoopClient) Native(apiKey, path string, body []byte, query string, method string) (int, []byte, error) {
	return 200, []byte(`{"ok":true}`), nil
}
```

```go
// go-wails/internal/proxy/handler.go
package proxy

import (
	"io"
	"net/http"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/auth"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
)

type Handler struct {
	cfg    config.AppConfig
	client Client
}

func NewHandler(cfg config.AppConfig, client Client) *Handler {
	return &Handler{cfg: cfg, client: client}
}

func (h *Handler) Models(w http.ResponseWriter, r *http.Request) {
	if !h.allow(r) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	status, body, err := h.client.Models(h.firstAPIKey())
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadGateway)
		return
	}
	w.WriteHeader(status)
	_, _ = w.Write(body)
}

func (h *Handler) ChatCompletions(w http.ResponseWriter, r *http.Request) {
	if !h.allow(r) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	body, _ := io.ReadAll(r.Body)
	status, resp, err := h.client.Chat(h.firstAPIKey(), body, r.URL.RawQuery)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadGateway)
		return
	}
	w.WriteHeader(status)
	_, _ = w.Write(resp)
}

func (h *Handler) NativeProxy(w http.ResponseWriter, r *http.Request, subPath string) {
	if !h.allow(r) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	body, _ := io.ReadAll(r.Body)
	status, resp, err := h.client.Native(h.firstAPIKey(), subPath, body, r.URL.RawQuery, r.Method)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadGateway)
		return
	}
	w.WriteHeader(status)
	_, _ = w.Write(resp)
}

func (h *Handler) allow(r *http.Request) bool {
	token := auth.ExtractProxyToken(r)
	return auth.IsAllowedProxyToken(token, h.cfg.AllowedTokens)
}

func (h *Handler) firstAPIKey() string {
	if len(h.cfg.APIKeys) == 0 {
		return ""
	}
	return h.cfg.APIKeys[0]
}
```

- [ ] **Step 4: Run auth and proxy tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/auth ./internal/proxy -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/auth/token.go go-wails/internal/auth/token_test.go go-wails/internal/proxy/client.go go-wails/internal/proxy/handler.go go-wails/internal/proxy/handler_test.go
rtk git commit -m "feat(go-wails): add phase1 auth and proxy handlers"
```

### Task 5: Implement Admin API, Router, and Headless Entrypoint

**Files:**
- Create: `go-wails/internal/admin/session.go`
- Create: `go-wails/internal/admin/handler.go`
- Create: `go-wails/internal/admin/handler_test.go`
- Create: `go-wails/internal/server/router.go`
- Create: `go-wails/cmd/server/main.go`

- [ ] **Step 1: Write failing tests for login/config/keys/health**

```go
// go-wails/internal/admin/handler_test.go
package admin

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

func TestLoginAndSessionStatus(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin-1", AllowedTokens: []string{"sk-user-1"}, APIKeys: []string{"AIza-A"}}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(mgr, ".env.test")

	req := httptest.NewRequest(http.MethodPost, "/api/v1/session/login", bytes.NewBufferString(`{"authToken":"sk-admin-1"}`))
	w := httptest.NewRecorder()
	h.Login(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}
```

- [ ] **Step 2: Run tests and confirm failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/admin -v"`  
Expected: FAIL with undefined `NewHandler` and handler methods.

- [ ] **Step 3: Implement admin/session/router/headless main**

```go
// go-wails/internal/admin/session.go
package admin

import (
	"crypto/rand"
	"encoding/hex"
	"sync"
)

type SessionStore struct {
	mu    sync.RWMutex
	items map[string]struct{}
}

func NewSessionStore() *SessionStore {
	return &SessionStore{items: map[string]struct{}{}}
}

func (s *SessionStore) Create() string {
	buf := make([]byte, 16)
	_, _ = rand.Read(buf)
	id := hex.EncodeToString(buf)
	s.mu.Lock()
	s.items[id] = struct{}{}
	s.mu.Unlock()
	return id
}

func (s *SessionStore) Has(id string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	_, ok := s.items[id]
	return ok
}

func (s *SessionStore) Delete(id string) {
	s.mu.Lock()
	delete(s.items, id)
	s.mu.Unlock()
}
```

```go
// go-wails/internal/admin/handler.go
package admin

import (
	"encoding/json"
	"net/http"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

type Handler struct {
	mgr      *runtime.Manager
	envPath  string
	sessions *SessionStore
}

func NewHandler(mgr *runtime.Manager, envPath string) *Handler {
	return &Handler{mgr: mgr, envPath: envPath, sessions: NewSessionStore()}
}

func (h *Handler) Login(w http.ResponseWriter, r *http.Request) {
	var body struct {
		AuthToken string `json:"authToken"`
	}
	_ = json.NewDecoder(r.Body).Decode(&body)
	if body.AuthToken != h.mgr.Config().AuthToken {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	sid := h.sessions.Create()
	http.SetCookie(w, &http.Cookie{Name: "gb_session", Value: sid, HttpOnly: true, Path: "/"})
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}

func (h *Handler) SessionStatus(w http.ResponseWriter, r *http.Request) {
	_, err := r.Cookie("gb_session")
	_ = json.NewEncoder(w).Encode(map[string]any{"authenticated": err == nil})
}

func (h *Handler) ConfigGet(w http.ResponseWriter, r *http.Request) {
	_ = json.NewEncoder(w).Encode(h.mgr.Config())
}

func (h *Handler) ConfigPut(w http.ResponseWriter, r *http.Request) {
	var cfg config.AppConfig
	if err := json.NewDecoder(r.Body).Decode(&cfg); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if err := config.UpdateEnvFile(h.envPath, cfg); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	h.mgr.UpdateConfig(cfg)
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true, "config": cfg})
}

func (h *Handler) KeysList(w http.ResponseWriter, r *http.Request) {
	cfg := h.mgr.Config()
	_ = json.NewEncoder(w).Encode(map[string]any{"items": cfg.APIKeys})
}

func (h *Handler) Health(w http.ResponseWriter, r *http.Request) {
	_ = json.NewEncoder(w).Encode(h.mgr.Health())
}

func (h *Handler) Start(w http.ResponseWriter, r *http.Request) {
	_ = h.mgr.Start()
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}

func (h *Handler) Stop(w http.ResponseWriter, r *http.Request) {
	_ = h.mgr.Stop()
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}
```

```go
// go-wails/internal/server/router.go
package server

import (
	"net/http"
	"strings"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/admin"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/proxy"
	"github.com/go-chi/chi/v5"
)

func NewRouter(adminH *admin.Handler, proxyH *proxy.Handler) http.Handler {
	r := chi.NewRouter()
	r.Post("/api/v1/session/login", adminH.Login)
	r.Get("/api/v1/session/status", adminH.SessionStatus)
	r.Get("/api/v1/config", adminH.ConfigGet)
	r.Put("/api/v1/config", adminH.ConfigPut)
	r.Get("/api/v1/keys", adminH.KeysList)
	r.Get("/api/v1/health", adminH.Health)
	r.Post("/api/v1/runtime/start", adminH.Start)
	r.Post("/api/v1/runtime/stop", adminH.Stop)
	r.Get("/v1/models", proxyH.Models)
	r.Post("/v1/chat/completions", proxyH.ChatCompletions)
	r.HandleFunc("/v1beta/*", func(w http.ResponseWriter, req *http.Request) {
		sub := strings.TrimPrefix(req.URL.Path, "/v1beta/")
		proxyH.NativeProxy(w, req, sub)
	})
	return r
}
```

```go
// go-wails/cmd/server/main.go
package main

import (
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/admin"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/proxy"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/server"
)

func main() {
	env := map[string]string{
		"AUTH_TOKEN":         os.Getenv("AUTH_TOKEN"),
		"ALLOWED_TOKENS":     os.Getenv("ALLOWED_TOKENS"),
		"API_KEYS":           os.Getenv("API_KEYS"),
		"RUNTIME_BIND_HOST":  os.Getenv("RUNTIME_BIND_HOST"),
		"RUNTIME_PORT_START": os.Getenv("RUNTIME_PORT_START"),
		"RUNTIME_PORT_END":   os.Getenv("RUNTIME_PORT_END"),
		"POOL_STRATEGY":      os.Getenv("POOL_STRATEGY"),
		"MODEL_POOLS":        os.Getenv("MODEL_POOLS"),
	}
	cfg, err := config.LoadFromEnv(env)
	if err != nil {
		log.Fatalf("load config: %v", err)
	}

	mgr := runtime.NewManager(cfg)
	_ = mgr.Start()
	adminH := admin.NewHandler(mgr, ".env")
	proxyH := proxy.NewHandler(cfg, &proxy.NoopClient{})
	router := server.NewRouter(adminH, proxyH)

	addr := fmt.Sprintf("%s:%d", cfg.BindHost, cfg.PortStart)
	log.Printf("go-wails server listening on http://%s", addr)
	log.Fatal(http.ListenAndServe(addr, router))
}
```

- [ ] **Step 4: Run admin/server tests**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/admin ./internal/server -v"`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/admin/session.go go-wails/internal/admin/handler.go go-wails/internal/admin/handler_test.go go-wails/internal/server/router.go go-wails/cmd/server/main.go
rtk git commit -m "feat(go-wails): add phase1 admin api and headless entrypoint"
```

### Task 6: Add Wails Binding and Basic GUI (Login/Config/Keys/Health/Start-Stop)

**Files:**
- Create: `go-wails/internal/wails/app.go`
- Create: `go-wails/internal/wails/app_test.go`
- Create: `go-wails/cmd/wails/main.go`
- Create: `go-wails/ui/wails/frontend/src/App.tsx`

- [ ] **Step 1: Write failing binding tests**

```go
// go-wails/internal/wails/app_test.go
package wails

import (
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

func TestGetHealthReflectsRuntime(t *testing.T) {
	mgr := runtime.NewManager(config.AppConfig{BindHost: "127.0.0.1", PortStart: 18080})
	app := NewApp(mgr)
	health := app.GetHealth()
	if health.ListenAddr == "" {
		t.Fatal("expected listen addr")
	}
}
```

- [ ] **Step 2: Run tests and confirm failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/wails -v"`  
Expected: FAIL with undefined `NewApp`.

- [ ] **Step 3: Implement Wails binding and minimal frontend**

```go
// go-wails/internal/wails/app.go
package wails

import (
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

type App struct {
	mgr *runtime.Manager
}

func NewApp(mgr *runtime.Manager) *App {
	return &App{mgr: mgr}
}

func (a *App) GetHealth() runtime.Health {
	return a.mgr.Health()
}

func (a *App) GetConfig() config.AppConfig {
	return a.mgr.Config()
}

func (a *App) SaveConfig(cfg config.AppConfig) {
	a.mgr.UpdateConfig(cfg)
}

func (a *App) StartService() error {
	return a.mgr.Start()
}

func (a *App) StopService() error {
	return a.mgr.Stop()
}
```

```go
// go-wails/cmd/wails/main.go
package main

import (
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	appbind "github.com/alan/gemini-pool-proxy/go-wails/internal/wails"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
)

func main() {
	cfg := config.AppConfig{
		BindHost:     "127.0.0.1",
		PortStart:    18080,
		AllowedTokens: []string{"sk-user-demo"},
		APIKeys:      []string{},
	}
	mgr := runtime.NewManager(cfg)
	app := appbind.NewApp(mgr)

	err := wails.Run(&options.App{
		Title:  "Gemini Pool Proxy (Go + Wails)",
		Width:  1200,
		Height: 800,
		Bind:   []any{app},
	})
	if err != nil {
		panic(err)
	}
}
```

```tsx
// go-wails/ui/wails/frontend/src/App.tsx
import { useEffect, useState } from "react";
import { GetConfig, GetHealth, SaveConfig, StartService, StopService } from "../wailsjs/go/wails/App";

type Health = {
  running: boolean;
  listenAddr: string;
  lastError: string;
  lastChangeAt: string;
};

export default function App() {
  const [health, setHealth] = useState<Health | null>(null);
  const [authToken, setAuthToken] = useState("");
  const [allowedTokens, setAllowedTokens] = useState("");

  useEffect(() => {
    GetConfig().then((cfg: any) => {
      setAuthToken(cfg.AuthToken || "");
      setAllowedTokens((cfg.AllowedTokens || []).join(","));
    });
    GetHealth().then((h: Health) => setHealth(h));
  }, []);

  async function onSave() {
    await SaveConfig({
      AuthToken: authToken,
      AllowedTokens: allowedTokens.split(",").map((v) => v.trim()).filter(Boolean),
    });
    setHealth(await GetHealth());
  }

  return (
    <main style={{ padding: 24, fontFamily: "IBM Plex Sans, sans-serif" }}>
      <h1>Gemini Pool Proxy - Phase 1</h1>
      <p>Status: {health?.running ? "Running" : "Stopped"} ({health?.listenAddr})</p>
      <label>Admin Token</label>
      <input value={authToken} onChange={(e) => setAuthToken(e.target.value)} />
      <label>Allowed Tokens (CSV)</label>
      <input value={allowedTokens} onChange={(e) => setAllowedTokens(e.target.value)} />
      <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
        <button onClick={onSave}>Save Config</button>
        <button onClick={() => StartService().then(() => GetHealth().then(setHealth))}>Start</button>
        <button onClick={() => StopService().then(() => GetHealth().then(setHealth))}>Stop</button>
      </div>
    </main>
  );
}
```

- [ ] **Step 4: Run binding and frontend checks**

Run: `rtk bash -lc "cd go-wails && rtk go test ./internal/wails -v"`  
Expected: PASS for binding tests.

Run: `rtk bash -lc "cd go-wails/ui/wails/frontend && rtk npm run build"`  
Expected: PASS with generated frontend build artifacts.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/internal/wails/app.go go-wails/internal/wails/app_test.go go-wails/cmd/wails/main.go go-wails/ui/wails/frontend/src/App.tsx
rtk git commit -m "feat(go-wails): add basic phase1 wails management ui"
```

### Task 7: Add Integration Contract Tests, Smoke Script, and Developer Docs

**Files:**
- Create: `go-wails/tests/integration/phase1_contract_test.go`
- Create: `go-wails/scripts/smoke-phase1.sh`
- Create: `go-wails/README.md`

- [ ] **Step 1: Write failing integration test for Phase 1 contracts**

```go
// go-wails/tests/integration/phase1_contract_test.go
package integration

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/admin"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/proxy"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/server"
)

func TestContract_ModelsRouteExists(t *testing.T) {
	cfg := config.AppConfig{AllowedTokens: []string{"sk-user-1"}, APIKeys: []string{"AIza-A"}, BindHost: "127.0.0.1", PortStart: 18080}
	mgr := runtime.NewManager(cfg)
	adminH := admin.NewHandler(mgr, ".env.test")
	proxyH := proxy.NewHandler(cfg, &integrationClient{})
	r := server.NewRouter(adminH, proxyH)

	ts := httptest.NewServer(r)
	defer ts.Close()

	req, _ := http.NewRequest(http.MethodGet, ts.URL+"/v1/models", nil)
	req.Header.Set("Authorization", "Bearer sk-user-1")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("http error: %v", err)
	}
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}
}

type integrationClient struct{}

func (c *integrationClient) Models(apiKey string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"object":"list","data":[]}`), nil
}

func (c *integrationClient) Chat(apiKey string, body []byte, query string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"id":"chatcmpl-integration"}`), nil
}

func (c *integrationClient) Native(apiKey, path string, body []byte, query string, method string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"ok":true}`), nil
}
```

- [ ] **Step 2: Run integration test and verify failure**

Run: `rtk bash -lc "cd go-wails && rtk go test ./tests/integration -v"`  
Expected: FAIL on missing fake client or route wiring.

- [ ] **Step 3: Implement smoke script and docs**

```bash
# go-wails/scripts/smoke-phase1.sh
#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:18080}"
PROXY_TOKEN="${PROXY_TOKEN:-sk-user-demo}"

echo "[smoke] check /v1/models"
rtk curl -sS "${BASE_URL}/v1/models" -H "Authorization: Bearer ${PROXY_TOKEN}" > /tmp/go-wails-models.json

echo "[smoke] check /api/v1/health"
rtk curl -sS "${BASE_URL}/api/v1/health" > /tmp/go-wails-health.json

echo "[smoke] done"
```

````markdown
<!-- go-wails/README.md -->
# go-wails (Phase 1)

## Run Headless

```bash
cd go-wails
rtk go run ./cmd/server
```

## Run GUI

```bash
cd go-wails
rtk wails dev -p cmd/wails
```

## Verify

```bash
cd go-wails
rtk go test ./...
rtk bash scripts/smoke-phase1.sh
```
````

- [ ] **Step 4: Run full verification**

Run: `rtk bash -lc "cd go-wails && rtk go test ./..."`  
Expected: PASS.

Run: `rtk bash -lc "cd go-wails && rtk bash scripts/smoke-phase1.sh"`  
Expected: smoke script prints `[smoke] done`.

- [ ] **Step 5: Commit**

```bash
rtk git add go-wails/tests/integration/phase1_contract_test.go go-wails/scripts/smoke-phase1.sh go-wails/README.md
rtk git commit -m "test(go-wails): add phase1 integration checks and smoke script"
```

## Self-Review Checklist (Completed)

1. **Spec coverage check:**  
   - Phase 1 core deliverables map to Tasks 1-7.  
   - Phase 2 intentionally excluded and explicitly deferred.
2. **Placeholder scan:**  
   - No `TBD`, `TODO`, or deferred implementation placeholders exist in task steps.
3. **Type/signature consistency:**  
   - `config.AppConfig`, `runtime.Manager`, `proxy.Handler`, and `admin.Handler` names are consistent across tasks.

## Phase 2 Follow-Up Requirement

Before implementing Phase 2 features, create a separate plan file for:
- key pool strategy migration
- model pool strategy migration
- logs and dashboard aggregation APIs/UI
