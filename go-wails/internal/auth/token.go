package auth

import (
	"net/http"
	"strings"
)

func ExtractProxyToken(r *http.Request) string {
	if auth := strings.TrimSpace(r.Header.Get("Authorization")); strings.HasPrefix(strings.ToLower(auth), "bearer ") {
		return strings.TrimSpace(auth[7:])
	}
	if v := strings.TrimSpace(r.Header.Get("x-api-key")); v != "" {
		return v
	}
	if v := strings.TrimSpace(r.Header.Get("x-goog-api-key")); v != "" {
		return v
	}
	if v := strings.TrimSpace(r.URL.Query().Get("key")); v != "" {
		return v
	}
	return ""
}

func IsAllowedProxyToken(token string, allowed []string) bool {
	for _, item := range allowed {
		if token == item {
			return true
		}
	}
	return false
}
