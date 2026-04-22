package integration

import (
	"bytes"
	"encoding/json"
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

func TestPhase3_KeyActionsAndUsageRoutes(t *testing.T) {
	cfg := config.AppConfig{
		AuthToken:     "sk-admin-1",
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"k1", "k2"},
		BindHost:      "127.0.0.1",
		PortStart:     18080,
	}
	mgr := runtime.NewManager(cfg)
	adminH := admin.NewHandler(mgr, ".env.test")
	proxyH := proxy.NewHandler(cfg, &proxy.NoopClient{}, mgr)
	router := server.NewRouter(adminH, proxyH)
	ts := httptest.NewServer(router)
	defer ts.Close()

	body := bytes.NewBufferString(`{"action":"reset","keyType":"all"}`)
	req, err := http.NewRequest(http.MethodPost, ts.URL+"/api/v1/keys/actions", body)
	if err != nil {
		t.Fatalf("build request failed: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	resp1, err := http.DefaultClient.Do(req)
	if err != nil || resp1.StatusCode != http.StatusOK {
		if resp1 != nil {
			t.Fatalf("keys actions route failed, status=%v err=%v", resp1.StatusCode, err)
		}
		t.Fatalf("keys actions route failed, err=%v", err)
	}

	resp2, err := http.Get(ts.URL + "/api/v1/keys/usage/key-1?period=24h")
	if err != nil || resp2.StatusCode != http.StatusOK {
		if resp2 != nil {
			t.Fatalf("keys usage route failed, status=%v err=%v", resp2.StatusCode, err)
		}
		t.Fatalf("keys usage route failed, err=%v", err)
	}
	var payload map[string]any
	if err := json.NewDecoder(resp2.Body).Decode(&payload); err != nil {
		t.Fatalf("decode usage response failed: %v", err)
	}
	key, _ := payload["key"].(string)
	if key == "" {
		t.Fatalf("expected key in response, got: %#v", payload)
	}
}
