package runtime

import (
	"fmt"
	"sync"
	"time"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/keypool"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/metrics"
	"github.com/alan/gemini-pool-proxy/go-wails/internal/modelpool"
)

type Health struct {
	Running      bool   `json:"running"`
	ListenAddr   string `json:"listenAddr"`
	LastError    string `json:"lastError"`
	LastChangeAt string `json:"lastChangeAt"`
}

type Manager struct {
	mu         sync.RWMutex
	cfg        config.AppConfig
	running    bool
	lastError  string
	lastChange time.Time
	keyPool    *keypool.Pool
	modelPool  *modelpool.Pool
	metrics    *metrics.Store
}

func NewManager(cfg config.AppConfig) *Manager {
	maxFailures := cfg.MaxFailures
	if maxFailures <= 0 {
		maxFailures = 3
	}
	cooldown := cfg.CooldownSeconds
	if cooldown <= 0 {
		cooldown = 60
	}
	return &Manager{
		cfg:        cfg,
		lastChange: time.Now(),
		keyPool:    keypool.NewPool(cfg.APIKeys, maxFailures, cooldown, cfg.PoolStrategy),
		modelPool:  modelpool.NewPool(cfg.ModelPools, cfg.ModelPoolStrategy),
		metrics:    metrics.NewStore(2000),
	}
}

func (m *Manager) Start() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.running = true
	m.lastError = ""
	m.lastChange = time.Now()
	return nil
}

func (m *Manager) Stop() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.running = false
	m.lastChange = time.Now()
	return nil
}

func (m *Manager) UpdateConfig(cfg config.AppConfig) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.cfg = cfg
	maxFailures := cfg.MaxFailures
	if maxFailures <= 0 {
		maxFailures = 3
	}
	cooldown := cfg.CooldownSeconds
	if cooldown <= 0 {
		cooldown = 60
	}
	m.keyPool = keypool.NewPool(cfg.APIKeys, maxFailures, cooldown, cfg.PoolStrategy)
	m.modelPool = modelpool.NewPool(cfg.ModelPools, cfg.ModelPoolStrategy)
	m.lastChange = time.Now()
}

func (m *Manager) Config() config.AppConfig {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.cfg
}

func (m *Manager) Health() Health {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return Health{
		Running:      m.running,
		ListenAddr:   fmt.Sprintf("%s:%d", m.cfg.BindHost, m.cfg.PortStart),
		LastError:    m.lastError,
		LastChangeAt: m.lastChange.UTC().Format(time.RFC3339),
	}
}

func (m *Manager) KeyPool() *keypool.Pool {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.keyPool
}

func (m *Manager) ModelPool() *modelpool.Pool {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.modelPool
}

func (m *Manager) Metrics() *metrics.Store {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.metrics
}

func (m *Manager) SetPoolStrategy(strategy string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if strategy == "" {
		return
	}
	m.cfg.PoolStrategy = strategy
	m.keyPool.SetStrategy(strategy)
	m.lastChange = time.Now()
}
