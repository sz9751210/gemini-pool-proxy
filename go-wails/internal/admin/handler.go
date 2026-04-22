package admin

import (
	"encoding/json"
	"net/http"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
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
