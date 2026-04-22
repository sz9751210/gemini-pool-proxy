package main

import (
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
	appbind "github.com/alan/gemini-pool-proxy/go-wails/internal/wails"
	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
)

func main() {
	cfg := config.AppConfig{
		BindHost:      "127.0.0.1",
		PortStart:     18080,
		AllowedTokens: []string{"sk-user-demo"},
		APIKeys:       []string{},
	}
	mgr := runtime.NewManager(cfg)
	app := appbind.NewApp(mgr)

	err := wails.Run(&options.App{
		Title:  "Gemini Pool Proxy (Go + Wails)",
		Width:  1200,
		Height: 800,
		Bind:   []any{app},
	})
	if err != nil {
		panic(err)
	}
}
