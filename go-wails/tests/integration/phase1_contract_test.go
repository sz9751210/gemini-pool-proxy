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
	cfg := config.AppConfig{
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"AIza-A"},
		BindHost:      "127.0.0.1",
		PortStart:     18080,
	}
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
