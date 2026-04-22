package config

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

type AppConfig struct {
	AuthToken         string
	AllowedTokens     []string
	APIKeys           []string
	BindHost          string
	PortStart         int
	PortEnd           int
	PoolStrategy      string
	ModelPools        map[string][]string
	MaxFailures       int
	CooldownSeconds   int
	ModelPoolStrategy string
	ModelPoolScope    string
}

func LoadFromEnv(env map[string]string) (AppConfig, error) {
	start, err := atoiDefault(env["RUNTIME_PORT_START"], 18080)
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid RUNTIME_PORT_START: %w", err)
	}
	end, err := atoiDefault(env["RUNTIME_PORT_END"], 18099)
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid RUNTIME_PORT_END: %w", err)
	}
	modelPools, err := parseModelPools(env["MODEL_POOLS"])
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid MODEL_POOLS: %w", err)
	}
	maxFailures, err := atoiDefault(env["MAX_FAILURES"], 3)
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid MAX_FAILURES: %w", err)
	}
	cooldownSecs, err := atoiDefault(env["COOLDOWN_SECONDS"], 60)
	if err != nil {
		return AppConfig{}, fmt.Errorf("invalid COOLDOWN_SECONDS: %w", err)
	}
	return AppConfig{
		AuthToken:         valueOr(env["AUTH_TOKEN"], "sk-admin-demo"),
		AllowedTokens:     parseArray(valueOr(env["ALLOWED_TOKENS"], "sk-user-demo")),
		APIKeys:           parseArray(valueOr(env["API_KEYS"], "")),
		BindHost:          valueOr(env["RUNTIME_BIND_HOST"], "127.0.0.1"),
		PortStart:         start,
		PortEnd:           end,
		PoolStrategy:      valueOr(env["POOL_STRATEGY"], "round_robin"),
		ModelPools:        modelPools,
		MaxFailures:       maxFailures,
		CooldownSeconds:   cooldownSecs,
		ModelPoolStrategy: valueOr(env["MODEL_POOL_STRATEGY"], "round_robin"),
		ModelPoolScope:    valueOr(env["MODEL_POOL_SCOPE"], "global"),
	}, nil
}

func parseArray(raw string) []string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return []string{}
	}
	if strings.HasPrefix(raw, "[") && strings.HasSuffix(raw, "]") {
		var out []string
		if err := json.Unmarshal([]byte(raw), &out); err == nil {
			return trimItems(out)
		}
		raw = strings.TrimSuffix(strings.TrimPrefix(raw, "["), "]")
	}
	return trimItems(strings.Split(raw, ","))
}

func parseModelPools(raw string) (map[string][]string, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return map[string][]string{}, nil
	}
	out := map[string][]string{}
	if err := json.Unmarshal([]byte(raw), &out); err == nil {
		return out, nil
	}
	return nil, fmt.Errorf("expected JSON object")
}

func atoiDefault(raw string, fallback int) (int, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return fallback, nil
	}
	return strconv.Atoi(raw)
}

func valueOr(v, fallback string) string {
	v = strings.TrimSpace(v)
	if v == "" {
		return fallback
	}
	return v
}

func trimItems(items []string) []string {
	out := make([]string, 0, len(items))
	for _, item := range items {
		s := strings.TrimSpace(strings.Trim(item, `"'`))
		if s != "" {
			out = append(out, s)
		}
	}
	return out
}
