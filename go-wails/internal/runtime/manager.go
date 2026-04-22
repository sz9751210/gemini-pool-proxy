package runtime

import (
	"fmt"
	"sync"
	"time"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
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
}

func NewManager(cfg config.AppConfig) *Manager {
	return &Manager{
		cfg:        cfg,
		lastChange: time.Now(),
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
