package main

import (
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/admin"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/proxy"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/server"
)

func main() {
	env := map[string]string{
		"AUTH_TOKEN":          os.Getenv("AUTH_TOKEN"),
		"ALLOWED_TOKENS":      os.Getenv("ALLOWED_TOKENS"),
		"API_KEYS":            os.Getenv("API_KEYS"),
		"RUNTIME_BIND_HOST":   os.Getenv("RUNTIME_BIND_HOST"),
		"RUNTIME_PORT_START":  os.Getenv("RUNTIME_PORT_START"),
		"RUNTIME_PORT_END":    os.Getenv("RUNTIME_PORT_END"),
		"POOL_STRATEGY":       os.Getenv("POOL_STRATEGY"),
		"MODEL_POOLS":         os.Getenv("MODEL_POOLS"),
		"MAX_FAILURES":        os.Getenv("MAX_FAILURES"),
		"COOLDOWN_SECONDS":    os.Getenv("COOLDOWN_SECONDS"),
		"MODEL_POOL_STRATEGY": os.Getenv("MODEL_POOL_STRATEGY"),
		"MODEL_POOL_SCOPE":    os.Getenv("MODEL_POOL_SCOPE"),
	}
	cfg, err := config.LoadFromEnv(env)
	if err != nil {
		log.Fatalf("load config: %v", err)
	}

	mgr := runtime.NewManager(cfg)
	_ = mgr.Start()
	adminH := admin.NewHandler(mgr, ".env")
	proxyH := proxy.NewHandler(cfg, &proxy.NoopClient{}, mgr)
	router := server.NewRouter(adminH, proxyH)

	addr := fmt.Sprintf("%s:%d", cfg.BindHost, cfg.PortStart)
	log.Printf("go-wails server listening on http://%s", addr)
	log.Fatal(http.ListenAndServe(addr, router))
}
