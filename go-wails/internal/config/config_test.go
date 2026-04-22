package config

import "testing"

func TestLoadFromEnv_ParsesCoreFields(t *testing.T) {
	env := map[string]string{
		"AUTH_TOKEN":         "sk-admin-1",
		"ALLOWED_TOKENS":     `["sk-user-1","sk-user-2"]`,
		"API_KEYS":           "AIza-A,AIza-B",
		"RUNTIME_BIND_HOST":  "127.0.0.1",
		"RUNTIME_PORT_START": "18080",
		"RUNTIME_PORT_END":   "18099",
		"POOL_STRATEGY":      "round_robin",
		"MODEL_POOLS":        `{"fast":["gemini-2.5-flash"]}`,
	}

	cfg, err := LoadFromEnv(env)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.AuthToken != "sk-admin-1" {
		t.Fatalf("wrong auth token: %s", cfg.AuthToken)
	}
	if len(cfg.AllowedTokens) != 2 || cfg.AllowedTokens[0] != "sk-user-1" {
		t.Fatalf("allowed tokens parse failed: %#v", cfg.AllowedTokens)
	}
	if len(cfg.APIKeys) != 2 || cfg.APIKeys[1] != "AIza-B" {
		t.Fatalf("api keys parse failed: %#v", cfg.APIKeys)
	}
	if cfg.BindHost != "127.0.0.1" || cfg.PortStart != 18080 || cfg.PortEnd != 18099 {
		t.Fatalf("listen config parse failed: %#v", cfg)
	}
	if cfg.ModelPools["fast"][0] != "gemini-2.5-flash" {
		t.Fatalf("model pool parse failed: %#v", cfg.ModelPools)
	}
}

func TestParseArray_AcceptsLooseAndJSON(t *testing.T) {
	a := parseArray(`["x","y"]`)
	b := parseArray("x,y")
	if len(a) != 2 || a[1] != "y" {
		t.Fatalf("json array parse failed: %#v", a)
	}
	if len(b) != 2 || b[0] != "x" {
		t.Fatalf("csv parse failed: %#v", b)
	}
}

func TestLoadFromEnv_ParsesPhase2Fields(t *testing.T) {
	env := map[string]string{
		"MAX_FAILURES":        "5",
		"COOLDOWN_SECONDS":    "90",
		"MODEL_POOL_STRATEGY": "per_key_cycle",
		"MODEL_POOL_SCOPE":    "global",
	}
	cfg, err := LoadFromEnv(env)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.MaxFailures != 5 || cfg.CooldownSeconds != 90 {
		t.Fatalf("numeric fields mismatch: %#v", cfg)
	}
	if cfg.ModelPoolStrategy != "per_key_cycle" || cfg.ModelPoolScope != "global" {
		t.Fatalf("strategy fields mismatch: %#v", cfg)
	}
}
