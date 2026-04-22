package config

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strings"
)

func UpdateEnvFile(path string, cfg AppConfig) error {
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	lines := strings.Split(string(raw), "\n")

	desired := map[string]string{
		"AUTH_TOKEN":          cfg.AuthToken,
		"ALLOWED_TOKENS":      mustJSON(cfg.AllowedTokens),
		"API_KEYS":            mustJSON(cfg.APIKeys),
		"RUNTIME_BIND_HOST":   cfg.BindHost,
		"RUNTIME_PORT_START":  fmt.Sprintf("%d", cfg.PortStart),
		"RUNTIME_PORT_END":    fmt.Sprintf("%d", cfg.PortEnd),
		"POOL_STRATEGY":       cfg.PoolStrategy,
		"MODEL_POOLS":         mustJSON(cfg.ModelPools),
		"MAX_FAILURES":        fmt.Sprintf("%d", cfg.MaxFailures),
		"COOLDOWN_SECONDS":    fmt.Sprintf("%d", cfg.CooldownSeconds),
		"MODEL_POOL_STRATEGY": cfg.ModelPoolStrategy,
		"MODEL_POOL_SCOPE":    cfg.ModelPoolScope,
	}

	seen := map[string]bool{}
	for i, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") || !strings.Contains(trimmed, "=") {
			continue
		}
		key, _, ok := strings.Cut(trimmed, "=")
		if !ok {
			continue
		}
		key = strings.TrimSpace(key)
		if val, has := desired[key]; has {
			lines[i] = fmt.Sprintf("%s=%s", key, val)
			seen[key] = true
		}
	}

	missing := make([]string, 0, len(desired))
	for key := range desired {
		if !seen[key] {
			missing = append(missing, key)
		}
	}
	sort.Strings(missing)
	for _, key := range missing {
		lines = append(lines, fmt.Sprintf("%s=%s", key, desired[key]))
	}

	out := strings.TrimRight(strings.Join(lines, "\n"), "\n") + "\n"
	return os.WriteFile(path, []byte(out), 0o644)
}

func mustJSON(v any) string {
	body, _ := json.Marshal(v)
	return string(body)
}
