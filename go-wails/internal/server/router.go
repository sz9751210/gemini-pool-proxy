package server

import (
	"net/http"
	"strings"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/admin"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/proxy"
	"github.com/go-chi/chi/v5"
)

func NewRouter(adminH *admin.Handler, proxyH *proxy.Handler) http.Handler {
	r := chi.NewRouter()
	r.Post("/api/v1/session/login", adminH.Login)
	r.Post("/api/v1/session/logout", adminH.Logout)
	r.Get("/api/v1/session/status", adminH.SessionStatus)
	r.Get("/api/v1/config", adminH.ConfigGet)
	r.Put("/api/v1/config", adminH.ConfigPut)
	r.Get("/api/v1/keys", adminH.KeysList)
	r.Get("/api/v1/health", adminH.Health)
	r.Get("/api/v1/logs", adminH.Logs)
	r.Get("/api/v1/dashboard/overview", adminH.DashboardOverview)
	r.Get("/api/v1/pool/status", adminH.PoolStatus)
	r.Put("/api/v1/pool/strategy", adminH.UpdatePoolStrategy)
	r.Post("/api/v1/runtime/start", adminH.Start)
	r.Post("/api/v1/runtime/stop", adminH.Stop)
	r.Get("/v1/models", proxyH.Models)
	r.Post("/v1/chat/completions", proxyH.ChatCompletions)
	r.HandleFunc("/v1beta/*", func(w http.ResponseWriter, req *http.Request) {
		sub := strings.TrimPrefix(req.URL.Path, "/v1beta/")
		proxyH.NativeProxy(w, req, sub)
	})
	return r
}
