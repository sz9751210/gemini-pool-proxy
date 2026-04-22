package admin

import (
	"encoding/json"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/keypool"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	"github.com/go-chi/chi/v5"
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

func (h *Handler) Logout(w http.ResponseWriter, r *http.Request) {
	cookie, err := r.Cookie("gb_session")
	if err == nil {
		h.sessions.Delete(cookie.Value)
	}
	_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
}

func (h *Handler) SessionStatus(w http.ResponseWriter, r *http.Request) {
	cookie, err := r.Cookie("gb_session")
	if err != nil {
		_ = json.NewEncoder(w).Encode(map[string]any{"authenticated": false})
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]any{"authenticated": h.sessions.Has(cookie.Value)})
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

func (h *Handler) KeysActions(w http.ResponseWriter, r *http.Request) {
	var body struct {
		Action  string   `json:"action"`
		IDs     []string `json:"ids"`
		Keys    []string `json:"keys"`
		KeyType string   `json:"keyType"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	action := strings.ToLower(strings.TrimSpace(body.Action))
	if action != "verify" && action != "reset" && action != "delete" {
		http.Error(w, "invalid action", http.StatusBadRequest)
		return
	}

	poolItems := h.mgr.KeyPool().Snapshot()
	targetIDs, failedItems := parseKeyActionTargets(poolItems, body.IDs, body.Keys, body.KeyType)
	if len(targetIDs) == 0 {
		http.Error(w, "no targets", http.StatusBadRequest)
		return
	}

	switch action {
	case "verify", "reset":
		h.mgr.KeyPool().ResetFailuresByIDs(targetIDs)
	case "delete":
		h.mgr.KeyPool().RemoveByIDs(targetIDs)
		cfg := h.mgr.Config()
		cfg.APIKeys = h.mgr.KeyPool().RawKeys()
		if h.envPath != "" {
			if info, err := os.Stat(h.envPath); err == nil && !info.IsDir() {
				if err := config.UpdateEnvFile(h.envPath, cfg); err != nil {
					http.Error(w, err.Error(), http.StatusInternalServerError)
					return
				}
			}
		}
		h.mgr.UpdateConfig(cfg)
	}
	successCount := len(targetIDs) - len(failedItems)
	if successCount < 0 {
		successCount = 0
	}

	_ = json.NewEncoder(w).Encode(map[string]any{
		"action":       action,
		"successCount": successCount,
		"failedItems":  failedItems,
		"message":      action + " 已完成",
		"success":      true,
	})
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

func (h *Handler) DashboardOverview(w http.ResponseWriter, r *http.Request) {
	now := time.Now()
	_ = json.NewEncoder(w).Encode(h.mgr.Metrics().DashboardOverview(now))
}

func (h *Handler) Logs(w http.ResponseWriter, r *http.Request) {
	limit := parseIntQuery(r, "limit", 20)
	offset := parseIntQuery(r, "offset", 0)
	_ = json.NewEncoder(w).Encode(map[string]any{
		"logs":   h.mgr.Metrics().Logs(limit, offset),
		"limit":  limit,
		"offset": offset,
	})
}

func (h *Handler) KeyUsage(w http.ResponseWriter, r *http.Request) {
	key := chi.URLParam(r, "key")
	if key == "" {
		http.Error(w, "key required", http.StatusBadRequest)
		return
	}
	period := r.URL.Query().Get("period")
	if period == "" {
		period = "24h"
	}

	usage := h.mgr.Metrics().UsageByKey(key, period, time.Now(), func(id string) string {
		raw, ok := h.mgr.KeyPool().RawKeyByID(id)
		if !ok {
			return ""
		}
		return raw
	})

	_ = json.NewEncoder(w).Encode(map[string]any{
		"key":    key,
		"period": period,
		"usage":  usage,
	})
}

func (h *Handler) PoolStatus(w http.ResponseWriter, r *http.Request) {
	_ = json.NewEncoder(w).Encode(map[string]any{
		"strategy": h.mgr.KeyPool().Strategy(),
		"keys":     h.mgr.KeyPool().Snapshot(),
	})
}

func (h *Handler) UpdatePoolStrategy(w http.ResponseWriter, r *http.Request) {
	var body struct {
		Strategy string `json:"strategy"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if body.Strategy == "" {
		http.Error(w, "strategy required", http.StatusBadRequest)
		return
	}
	h.mgr.SetPoolStrategy(body.Strategy)
	_ = json.NewEncoder(w).Encode(map[string]any{
		"ok":       true,
		"strategy": body.Strategy,
	})
}

func parseIntQuery(r *http.Request, key string, fallback int) int {
	raw := r.URL.Query().Get(key)
	if raw == "" {
		return fallback
	}
	v, err := strconv.Atoi(raw)
	if err != nil {
		return fallback
	}
	return v
}

func parseKeyActionTargets(poolItems []keypool.KeyState, providedIDs, providedKeys []string, keyType string) ([]string, []map[string]string) {
	ids := make([]string, 0, len(poolItems))
	failedItems := make([]map[string]string, 0)

	byID := make(map[string]string, len(poolItems))
	byRawKey := make(map[string]string, len(poolItems))
	for _, item := range poolItems {
		byID[item.ID] = item.ID
		byRawKey[item.RawKey] = item.ID
	}

	for _, id := range providedIDs {
		if mapped, ok := byID[id]; ok {
			ids = append(ids, mapped)
			continue
		}
		failedItems = append(failedItems, map[string]string{
			"key":    id,
			"reason": "找不到指定 id",
		})
	}
	for _, key := range providedKeys {
		if mapped, ok := byRawKey[key]; ok {
			ids = append(ids, mapped)
			continue
		}
		failedItems = append(failedItems, map[string]string{
			"key":    key,
			"reason": "找不到指定 key",
		})
	}

	mode := strings.TrimSpace(keyType)
	if mode == "" {
		mode = "all"
	}
	if len(ids) == 0 && len(failedItems) == 0 && strings.EqualFold(mode, "all") {
		for _, item := range poolItems {
			ids = append(ids, item.ID)
		}
	}

	seen := make(map[string]struct{}, len(ids))
	dedup := make([]string, 0, len(ids))
	for _, id := range ids {
		if _, ok := seen[id]; ok {
			continue
		}
		seen[id] = struct{}{}
		dedup = append(dedup, id)
	}
	return dedup, failedItems
}
