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
