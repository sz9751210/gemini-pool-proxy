package keypool

import (
	"sort"
	"sync"
	"time"
)

type KeyState struct {
	ID            string    `json:"id"`
	RawKey        string    `json:"rawKey"`
	FailureCount  int       `json:"failureCount"`
	CooldownUntil time.Time `json:"cooldownUntil"`
	LastUsedAt    time.Time `json:"lastUsedAt"`
}

type Pool struct {
	mu           sync.Mutex
	items        []KeyState
	next         int
	maxFailures  int
	cooldownSecs int
	strategy     string
	randomState  uint64
}

func NewPool(keys []string, maxFailures, cooldownSecs int, strategy string) *Pool {
	items := make([]KeyState, 0, len(keys))
	for i, key := range keys {
		items = append(items, KeyState{
			ID:     "key-" + itoa(i+1),
			RawKey: key,
		})
	}
	if maxFailures <= 0 {
		maxFailures = 3
	}
	if cooldownSecs <= 0 {
		cooldownSecs = 60
	}
	if strategy == "" {
		strategy = "round_robin"
	}
	return &Pool{
		items:        items,
		maxFailures:  maxFailures,
		cooldownSecs: cooldownSecs,
		strategy:     strategy,
		randomState:  1,
	}
}

func (p *Pool) SetStrategy(strategy string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if strategy == "" {
		return
	}
	p.strategy = strategy
}

func (p *Pool) Strategy() string {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.strategy
}

func (p *Pool) Next(token string, now time.Time) (KeyState, bool) {
	p.mu.Lock()
	defer p.mu.Unlock()

	p.recoverCooldown(now)
	if len(p.items) == 0 {
		return KeyState{}, false
	}

	idx := p.selectIndex(now)
	if idx < 0 {
		return KeyState{}, false
	}
	p.items[idx].LastUsedAt = now
	return p.items[idx], true
}

func (p *Pool) MarkFailure(rawKey string, now time.Time) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for i := range p.items {
		if p.items[i].RawKey == rawKey {
			p.items[i].FailureCount++
			if p.items[i].FailureCount >= p.maxFailures {
				p.items[i].CooldownUntil = now.Add(time.Duration(p.cooldownSecs) * time.Second)
			}
			return
		}
	}
}

func (p *Pool) MarkSuccess(rawKey string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for i := range p.items {
		if p.items[i].RawKey == rawKey {
			p.items[i].FailureCount = 0
			p.items[i].CooldownUntil = time.Time{}
			return
		}
	}
}

func (p *Pool) Snapshot() []KeyState {
	p.mu.Lock()
	defer p.mu.Unlock()
	out := make([]KeyState, len(p.items))
	copy(out, p.items)
	return out
}

func (p *Pool) ResetFailuresByIDs(ids []string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if len(ids) == 0 {
		return
	}
	target := make(map[string]struct{}, len(ids))
	for _, id := range ids {
		target[id] = struct{}{}
	}
	for i := range p.items {
		if _, ok := target[p.items[i].ID]; !ok {
			continue
		}
		p.items[i].FailureCount = 0
		p.items[i].CooldownUntil = time.Time{}
	}
}

func (p *Pool) RemoveByIDs(ids []string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if len(ids) == 0 {
		return
	}
	target := make(map[string]struct{}, len(ids))
	for _, id := range ids {
		target[id] = struct{}{}
	}
	nextItems := make([]KeyState, 0, len(p.items))
	for _, item := range p.items {
		if _, ok := target[item.ID]; ok {
			continue
		}
		nextItems = append(nextItems, item)
	}
	p.items = nextItems
	if len(p.items) == 0 {
		p.next = 0
		return
	}
	p.next %= len(p.items)
}

func (p *Pool) RawKeys() []string {
	p.mu.Lock()
	defer p.mu.Unlock()
	out := make([]string, 0, len(p.items))
	for _, item := range p.items {
		out = append(out, item.RawKey)
	}
	return out
}

func (p *Pool) RawKeyByID(id string) (string, bool) {
	p.mu.Lock()
	defer p.mu.Unlock()
	for _, item := range p.items {
		if item.ID == id {
			return item.RawKey, true
		}
	}
	return "", false
}

func (p *Pool) recoverCooldown(now time.Time) {
	for i := range p.items {
		if p.items[i].CooldownUntil.IsZero() {
			continue
		}
		if now.After(p.items[i].CooldownUntil) || now.Equal(p.items[i].CooldownUntil) {
			p.items[i].FailureCount = 0
			p.items[i].CooldownUntil = time.Time{}
		}
	}
}

func (p *Pool) activeIndexes(now time.Time) []int {
	indices := make([]int, 0, len(p.items))
	for i := range p.items {
		if p.items[i].CooldownUntil.IsZero() || now.After(p.items[i].CooldownUntil) || now.Equal(p.items[i].CooldownUntil) {
			indices = append(indices, i)
		}
	}
	return indices
}

func (p *Pool) selectIndex(now time.Time) int {
	switch p.strategy {
	case "random":
		return p.selectRandom(now)
	case "least_fail":
		return p.selectLeastFail(now)
	default:
		return p.selectRoundRobin(now)
	}
}

func (p *Pool) selectRoundRobin(now time.Time) int {
	active := p.activeIndexes(now)
	if len(active) == 0 {
		return -1
	}
	for i := 0; i < len(p.items); i++ {
		idx := (p.next + i) % len(p.items)
		if p.items[idx].CooldownUntil.IsZero() || now.After(p.items[idx].CooldownUntil) || now.Equal(p.items[idx].CooldownUntil) {
			p.next = (idx + 1) % len(p.items)
			return idx
		}
	}
	return -1
}

func (p *Pool) selectRandom(now time.Time) int {
	active := p.activeIndexes(now)
	if len(active) == 0 {
		return -1
	}
	p.randomState = p.randomState*1664525 + 1013904223 + uint64(now.UnixNano())
	return active[int(p.randomState%uint64(len(active)))]
}

func (p *Pool) selectLeastFail(now time.Time) int {
	active := p.activeIndexes(now)
	if len(active) == 0 {
		return -1
	}
	sort.SliceStable(active, func(i, j int) bool {
		a := p.items[active[i]]
		b := p.items[active[j]]
		if a.FailureCount != b.FailureCount {
			return a.FailureCount < b.FailureCount
		}
		if !a.LastUsedAt.Equal(b.LastUsedAt) {
			return a.LastUsedAt.Before(b.LastUsedAt)
		}
		return a.ID < b.ID
	})
	return active[0]
}

func itoa(v int) string {
	if v == 0 {
		return "0"
	}
	buf := [20]byte{}
	i := len(buf)
	for v > 0 {
		i--
		buf[i] = byte('0' + (v % 10))
		v /= 10
	}
	return string(buf[i:])
}
