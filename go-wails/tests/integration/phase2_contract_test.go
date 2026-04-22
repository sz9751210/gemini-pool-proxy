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
		if resp1 != nil {
			t.Fatalf("dashboard route failed, status=%v err=%v", resp1.StatusCode, err)
		}
		t.Fatalf("dashboard route failed, err=%v", err)
	}
	resp2, err := http.Get(ts.URL + "/api/v1/logs?limit=20&offset=0")
	if err != nil || resp2.StatusCode != http.StatusOK {
		if resp2 != nil {
			t.Fatalf("logs route failed, status=%v err=%v", resp2.StatusCode, err)
		}
		t.Fatalf("logs route failed, err=%v", err)
	}
}
