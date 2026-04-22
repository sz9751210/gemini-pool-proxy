package admin

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/metrics"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	"github.com/go-chi/chi/v5"
)

func TestLoginAndSessionStatus(t *testing.T) {
	cfg := config.AppConfig{
		AuthToken:     "sk-admin-1",
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"AIza-A"},
	}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(mgr, ".env.test")

	req := httptest.NewRequest(http.MethodPost, "/api/v1/session/login", bytes.NewBufferString(`{"authToken":"sk-admin-1"}`))
	w := httptest.NewRecorder()
	h.Login(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}

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

func TestKeysActions_ResetByID(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin", APIKeys: []string{"k1", "k2"}, MaxFailures: 2}
	mgr := runtime.NewManager(cfg)
	now := time.Now()
	mgr.KeyPool().MarkFailure("k1", now)

	h := NewHandler(mgr, ".env.test")
	req := httptest.NewRequest(http.MethodPost, "/api/v1/keys/actions", bytes.NewBufferString(`{"action":"reset","ids":["key-1"]}`))
	w := httptest.NewRecorder()
	h.KeysActions(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	snapshot := mgr.KeyPool().Snapshot()
	if len(snapshot) < 1 || snapshot[0].FailureCount != 0 {
		t.Fatalf("expected key-1 failure count reset, snapshot=%#v", snapshot)
	}
}

func TestKeysActions_DeleteByID(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin", APIKeys: []string{"k1", "k2"}}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(mgr, ".env.test")
	req := httptest.NewRequest(http.MethodPost, "/api/v1/keys/actions", bytes.NewBufferString(`{"action":"delete","ids":["key-2"]}`))
	w := httptest.NewRecorder()
	h.KeysActions(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	got := mgr.Config().APIKeys
	if len(got) != 1 || got[0] != "k1" {
		t.Fatalf("expected remaining keys [k1], got %#v", got)
	}
}

func TestKeysActions_InvalidAction(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin", APIKeys: []string{"k1"}}
	mgr := runtime.NewManager(cfg)
	h := NewHandler(mgr, ".env.test")
	req := httptest.NewRequest(http.MethodPost, "/api/v1/keys/actions", bytes.NewBufferString(`{"action":"noop","keyType":"all"}`))
	w := httptest.NewRecorder()
	h.KeysActions(w, req)
	if w.Code != http.StatusBadRequest {
		t.Fatalf("expected 400, got %d", w.Code)
	}
}

func TestKeyUsage_ByRawKey(t *testing.T) {
	cfg := config.AppConfig{AuthToken: "sk-admin", APIKeys: []string{"k1", "k2"}}
	mgr := runtime.NewManager(cfg)
	now := time.Now()
	mgr.Metrics().RecordCall(metrics.CallRecord{At: now.Add(-30 * time.Minute), KeyID: "key-1", Model: "m1", StatusCode: 200})
	mgr.Metrics().RecordCall(metrics.CallRecord{At: now.Add(-26 * time.Hour), KeyID: "key-1", Model: "m1", StatusCode: 200})
	mgr.Metrics().RecordCall(metrics.CallRecord{At: now.Add(-20 * time.Minute), KeyID: "key-2", Model: "m2", StatusCode: 200})

	h := NewHandler(mgr, ".env.test")
	req := httptest.NewRequest(http.MethodGet, "/api/v1/keys/usage/k1?period=24h", nil)
	routeCtx := chi.NewRouteContext()
	routeCtx.URLParams.Add("key", "k1")
	req = req.WithContext(context.WithValue(req.Context(), chi.RouteCtxKey, routeCtx))

	w := httptest.NewRecorder()
	h.KeyUsage(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	var payload struct {
		Key   string            `json:"key"`
		Usage map[string]uint64 `json:"usage"`
	}
	if err := json.NewDecoder(w.Body).Decode(&payload); err != nil {
		t.Fatalf("decode payload failed: %v", err)
	}
	if payload.Key != "k1" {
		t.Fatalf("expected key k1, got %q", payload.Key)
	}
	if payload.Usage["m1"] != 1 {
		t.Fatalf("expected usage m1=1, got %#v", payload.Usage)
	}
}
