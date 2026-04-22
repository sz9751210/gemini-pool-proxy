package proxy

import (
	"encoding/json"
	"io"
	"net/http"
	"time"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/auth"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/metrics"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

type Handler struct {
	cfg    config.AppConfig
	client Client
	mgr    *runtime.Manager
}

func NewHandler(cfg config.AppConfig, client Client, mgr *runtime.Manager) *Handler {
	return &Handler{cfg: cfg, client: client, mgr: mgr}
}

func (h *Handler) Models(w http.ResponseWriter, r *http.Request) {
	if !h.allow(r) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	key := h.pickKey("", time.Now())
	status, body, err := h.client.Models(key.RawKey)
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
	now := time.Now()
	body, _ := io.ReadAll(r.Body)
	token := auth.ExtractProxyToken(r)
	key := h.pickKey(token, now)
	if key.RawKey == "" {
		http.Error(w, "no key available", http.StatusServiceUnavailable)
		return
	}

	requestedModel := extractModel(body)
	actualModel := requestedModel
	if h.mgr != nil {
		actualModel = h.mgr.ModelPool().Resolve(requestedModel, key.ID)
	}
	body = replaceModel(body, actualModel)

	status, resp, err := h.client.Chat(key.RawKey, body, r.URL.RawQuery)
	if err != nil {
		h.markFailure(key.RawKey, now)
		h.record(now, key.ID, actualModel, http.StatusBadGateway, err.Error())
		http.Error(w, err.Error(), http.StatusBadGateway)
		return
	}
	if status >= 400 {
		h.markFailure(key.RawKey, now)
	} else {
		h.markSuccess(key.RawKey)
	}
	if h.mgr != nil {
		h.mgr.ModelPool().MarkUsed(requestedModel, actualModel)
	}
	h.record(now, key.ID, actualModel, status, "")
	w.WriteHeader(status)
	_, _ = w.Write(resp)
}

func (h *Handler) NativeProxy(w http.ResponseWriter, r *http.Request, subPath string) {
	if !h.allow(r) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	now := time.Now()
	body, _ := io.ReadAll(r.Body)
	token := auth.ExtractProxyToken(r)
	key := h.pickKey(token, now)
	status, resp, err := h.client.Native(key.RawKey, subPath, body, r.URL.RawQuery, r.Method)
	if err != nil {
		h.markFailure(key.RawKey, now)
		h.record(now, key.ID, subPath, http.StatusBadGateway, err.Error())
		http.Error(w, err.Error(), http.StatusBadGateway)
		return
	}
	if status >= 400 {
		h.markFailure(key.RawKey, now)
	} else {
		h.markSuccess(key.RawKey)
	}
	h.record(now, key.ID, subPath, status, "")
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

type selectedKey struct {
	ID     string
	RawKey string
}

func (h *Handler) pickKey(token string, now time.Time) selectedKey {
	if h.mgr != nil {
		key, ok := h.mgr.KeyPool().Next(token, now)
		if ok {
			return selectedKey{ID: key.ID, RawKey: key.RawKey}
		}
	}
	raw := h.firstAPIKey()
	return selectedKey{ID: "key-1", RawKey: raw}
}

func (h *Handler) markFailure(rawKey string, now time.Time) {
	if h.mgr == nil {
		return
	}
	h.mgr.KeyPool().MarkFailure(rawKey, now)
}

func (h *Handler) markSuccess(rawKey string) {
	if h.mgr == nil {
		return
	}
	h.mgr.KeyPool().MarkSuccess(rawKey)
}

func (h *Handler) record(now time.Time, keyID, model string, status int, detail string) {
	if h.mgr == nil {
		return
	}
	h.mgr.Metrics().RecordCall(metrics.CallRecord{
		At:         now,
		KeyID:      keyID,
		Model:      model,
		StatusCode: status,
		Detail:     detail,
	})
}

func extractModel(body []byte) string {
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		return ""
	}
	model, _ := payload["model"].(string)
	return model
}

func replaceModel(body []byte, model string) []byte {
	if model == "" {
		return body
	}
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		return body
	}
	payload["model"] = model
	updated, err := json.Marshal(payload)
	if err != nil {
		return body
	}
	return updated
}
