package proxy

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

func TestModelsEndpoint_RequiresAllowedToken(t *testing.T) {
	cfg := config.AppConfig{
		AllowedTokens: []string{"sk-user-1"},
		APIKeys:       []string{"AIza-A"},
	}
	h := NewHandler(cfg, &FakeClient{}, runtime.NewManager(cfg))
	req := httptest.NewRequest(http.MethodGet, "/v1/models", nil)
	req.Header.Set("Authorization", "Bearer wrong")
	w := httptest.NewRecorder()
	h.Models(w, req)
	if w.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d", w.Code)
	}
}

func TestChatCompletions_PassesBodyToClient(t *testing.T) {
	cfg := config.AppConfig{
		AllowedTokens:     []string{"sk-user-1"},
		APIKeys:           []string{"AIza-A"},
		ModelPools:        map[string][]string{"fast": []string{"m1", "m2"}},
		ModelPoolStrategy: "round_robin",
	}
	mgr := runtime.NewManager(cfg)
	client := &FakeClient{}
	h := NewHandler(cfg, client, mgr)
	req := httptest.NewRequest(http.MethodPost, "/v1/chat/completions", strings.NewReader(`{"model":"fast"}`))
	req.Header.Set("Authorization", "Bearer sk-user-1")
	w := httptest.NewRecorder()
	h.ChatCompletions(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	if len(mgr.Metrics().Logs(10, 0)) != 1 {
		t.Fatalf("expected one log record")
	}
	var payload map[string]any
	if err := json.Unmarshal(client.lastChatBody, &payload); err != nil {
		t.Fatalf("body should be valid json: %v", err)
	}
	gotModel, _ := payload["model"].(string)
	if gotModel == "" || gotModel == "fast" {
		t.Fatalf("expected alias resolved model, got %q", gotModel)
	}
}

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

type FakeClient struct {
	lastChatBody []byte
}

var _ Client = (*FakeClient)(nil)

func (f *FakeClient) Models(apiKey string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"object":"list","data":[]}`), nil
}

func (f *FakeClient) Chat(apiKey string, body []byte, query string) (int, []byte, error) {
	f.lastChatBody = make([]byte, len(body))
	copy(f.lastChatBody, body)
	return http.StatusOK, []byte(`{"id":"chatcmpl-test"}`), nil
}

func (f *FakeClient) Native(apiKey, path string, body []byte, query string, method string) (int, []byte, error) {
	return http.StatusOK, []byte(`{"ok":true}`), nil
}
