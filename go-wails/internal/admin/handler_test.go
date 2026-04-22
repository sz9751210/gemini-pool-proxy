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
