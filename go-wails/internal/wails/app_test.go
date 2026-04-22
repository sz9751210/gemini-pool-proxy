package wails

import (
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

func TestGetHealthReflectsRuntime(t *testing.T) {
	mgr := runtime.NewManager(config.AppConfig{BindHost: "127.0.0.1", PortStart: 18080})
	app := NewApp(mgr)
	health := app.GetHealth()
	if health.ListenAddr == "" {
		t.Fatal("expected listen addr")
	}
}
