package auth

import (
	"net/http/httptest"
	"testing"
)

func TestExtractProxyToken_BearerHeader(t *testing.T) {
	r := httptest.NewRequest("GET", "/", nil)
	r.Header.Set("Authorization", "Bearer sk-user-1")
	token := ExtractProxyToken(r)
	if token != "sk-user-1" {
		t.Fatalf("token mismatch: %s", token)
	}
}
