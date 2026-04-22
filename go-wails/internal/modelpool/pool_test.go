package modelpool

import "testing"

func TestModelPool_PerKeyCycle(t *testing.T) {
	p := NewPool(map[string][]string{"fast": []string{"m1", "m2"}}, "per_key_cycle")
	m1 := p.Resolve("fast", "key-a")
	m2 := p.Resolve("fast", "key-a")
	if m1 != m2 {
		t.Fatalf("same key in cycle should keep model, got %s/%s", m1, m2)
	}
	m3 := p.Resolve("fast", "key-b")
	if m3 == "" {
		t.Fatal("expected non-empty model")
	}
}

func TestModelPool_RoundRobin(t *testing.T) {
	p := NewPool(map[string][]string{"fast": []string{"m1", "m2"}}, "round_robin")
	a := p.Resolve("fast", "key-a")
	b := p.Resolve("fast", "key-b")
	if a == b {
		t.Fatalf("expected round robin model rotation, got %s", a)
	}
}

func TestModelPool_LeastUsed(t *testing.T) {
	p := NewPool(map[string][]string{"fast": []string{"m1", "m2"}}, "least_used")
	p.MarkUsed("fast", "m1")
	got := p.Resolve("fast", "key-a")
	if got != "m2" {
		t.Fatalf("expected m2 from least_used, got %s", got)
	}
}
