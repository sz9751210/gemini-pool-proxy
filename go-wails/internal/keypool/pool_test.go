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

func TestKeyPool_ResetFailuresByIDs(t *testing.T) {
	now := time.Now()
	p := NewPool([]string{"k1", "k2"}, 2, 60, "round_robin")
	p.MarkFailure("k1", now)
	p.MarkFailure("k1", now)
	p.ResetFailuresByIDs([]string{"key-1"})
	snapshot := p.Snapshot()
	if snapshot[0].FailureCount != 0 {
		t.Fatalf("expected key-1 failures reset, got %d", snapshot[0].FailureCount)
	}
	if !snapshot[0].CooldownUntil.IsZero() {
		t.Fatalf("expected key-1 cooldown cleared")
	}
}

func TestKeyPool_RemoveByIDs(t *testing.T) {
	p := NewPool([]string{"k1", "k2", "k3"}, 3, 60, "round_robin")
	p.RemoveByIDs([]string{"key-2"})
	snapshot := p.Snapshot()
	if len(snapshot) != 2 {
		t.Fatalf("expected 2 keys after delete, got %d", len(snapshot))
	}
	if snapshot[0].RawKey != "k1" || snapshot[1].RawKey != "k3" {
		t.Fatalf("unexpected remaining keys: %#v", snapshot)
	}
}
