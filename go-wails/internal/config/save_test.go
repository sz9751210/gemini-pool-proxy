package config

import (
	"os"
	"strings"
	"testing"
)

func TestUpdateEnvFile_PreservesUnknownLines(t *testing.T) {
	f, err := os.CreateTemp("", "phase1-env-*.env")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(f.Name())

	initial := "AUTH_TOKEN=old\nALLOWED_TOKENS=[\"a\"]\nCUSTOM_KEEP=1\n"
	if err := os.WriteFile(f.Name(), []byte(initial), 0o644); err != nil {
		t.Fatal(err)
	}

	cfg := AppConfig{
		AuthToken:     "new-admin",
		AllowedTokens: []string{"u1", "u2"},
		APIKeys:       []string{"k1"},
		BindHost:      "127.0.0.1",
		PortStart:     18080,
		PortEnd:       18099,
		PoolStrategy:  "round_robin",
		ModelPools:    map[string][]string{"fast": []string{"gemini-2.5-flash"}},
	}
	if err := UpdateEnvFile(f.Name(), cfg); err != nil {
		t.Fatalf("update failed: %v", err)
	}

	body, err := os.ReadFile(f.Name())
	if err != nil {
		t.Fatal(err)
	}
	text := string(body)
	if !strings.Contains(text, "AUTH_TOKEN=new-admin") {
		t.Fatalf("expected updated auth token, got: %s", text)
	}
	if !strings.Contains(text, "CUSTOM_KEEP=1") {
		t.Fatalf("expected unknown line preserved, got: %s", text)
	}
}
