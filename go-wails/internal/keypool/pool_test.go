package keypool

import (
	"testing"
	"time"
)

func TestKeyPool_RoundRobinRotation(t *testing.T) {
	p := NewPool([]string{"k1", "k2"}, 3, 60, "round_robin")
	first, ok := p.Next("user-a", time.Now())
	if !ok {
		t.Fatal("expected first key")
	}
	second, ok := p.Next("user-a", time.Now().Add(time.Second))
	if !ok {
		t.Fatal("expected second key")
	}
	if first.RawKey == second.RawKey {
		t.Fatalf("expected rotation, got same key %s", first.RawKey)
	}
}

func TestKeyPool_CooldownRecovery(t *testing.T) {
	now := time.Now()
	p := NewPool([]string{"k1"}, 2, 1, "round_robin")
	p.MarkFailure("k1", now)
	p.MarkFailure("k1", now)
	if _, ok := p.Next("u", now.Add(500*time.Millisecond)); ok {
		t.Fatal("expected key to be cooling down")
	}
	if _, ok := p.Next("u", now.Add(2*time.Second)); !ok {
		t.Fatal("expected key recovery after cooldown")
	}
}

func TestKeyPool_LeastFailPrefersLowerFailures(t *testing.T) {
	now := time.Now()
	p := NewPool([]string{"k1", "k2"}, 3, 60, "least_fail")
	p.MarkFailure("k1", now)
	selected, ok := p.Next("u", now.Add(time.Second))
	if !ok {
		t.Fatal("expected selection")
	}
	if selected.RawKey != "k2" {
		t.Fatalf("expected k2, got %s", selected.RawKey)
	}
}
