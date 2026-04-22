package runtime

import (
	"testing"

	"github.com/alan/gemini-pool-proxy/go-wails/internal/config"
)

func TestManager_StartStopAndHealth(t *testing.T) {
	m := NewManager(config.AppConfig{BindHost: "127.0.0.1", PortStart: 18080})
	if m.Health().Running {
		t.Fatal("should start stopped")
	}
	if err := m.Start(); err != nil {
		t.Fatalf("start failed: %v", err)
	}
	if !m.Health().Running {
		t.Fatal("expected running=true")
	}
	if err := m.Stop(); err != nil {
		t.Fatalf("stop failed: %v", err)
	}
	if m.Health().Running {
		t.Fatal("expected running=false")
	}
}
