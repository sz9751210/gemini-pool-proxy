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
