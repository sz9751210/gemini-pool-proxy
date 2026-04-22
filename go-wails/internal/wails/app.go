package wails

import (
	"time"

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

func (a *App) GetDashboardOverview() any {
	return a.mgr.Metrics().DashboardOverview(time.Now())
}

func (a *App) GetLogs(limit, offset int) any {
	return a.mgr.Metrics().Logs(limit, offset)
}

func (a *App) GetPoolStatus() any {
	return map[string]any{
		"strategy": a.mgr.KeyPool().Strategy(),
		"keys":     a.mgr.KeyPool().Snapshot(),
	}
}
