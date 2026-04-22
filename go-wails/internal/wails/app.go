package wails

import (
	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/runtime"
)

type App struct {
	mgr *runtime.Manager
}

func NewApp(mgr *runtime.Manager) *App {
	return &App{mgr: mgr}
}

func (a *App) GetHealth() runtime.Health {
	return a.mgr.Health()
}

func (a *App) GetConfig() config.AppConfig {
	return a.mgr.Config()
}

func (a *App) SaveConfig(cfg config.AppConfig) {
	a.mgr.UpdateConfig(cfg)
}

func (a *App) StartService() error {
	return a.mgr.Start()
}

func (a *App) StopService() error {
	return a.mgr.Stop()
}
