package modelpool

import "sync"

type Pool struct {
	mu        sync.Mutex
	aliases   map[string][]string
	strategy  string
	cursor    map[string]int
	usage     map[string]map[string]uint64
	lastByKey map[string]map[string]string
}

func NewPool(aliases map[string][]string, strategy string) *Pool {
	if strategy == "" {
		strategy = "round_robin"
	}
	copied := map[string][]string{}
	for k, v := range aliases {
		list := make([]string, len(v))
		copy(list, v)
		copied[k] = list
	}
	return &Pool{
		aliases:   copied,
		strategy:  strategy,
		cursor:    map[string]int{},
		usage:     map[string]map[string]uint64{},
		lastByKey: map[string]map[string]string{},
	}
}

func (p *Pool) Resolve(requestedModel, keyID string) string {
	p.mu.Lock()
	defer p.mu.Unlock()

	targets, ok := p.aliases[requestedModel]
	if !ok || len(targets) == 0 {
		return requestedModel
	}
	if _, ok := p.lastByKey[requestedModel]; !ok {
		p.lastByKey[requestedModel] = map[string]string{}
	}
	if _, ok := p.usage[requestedModel]; !ok {
		p.usage[requestedModel] = map[string]uint64{}
	}

	if p.strategy == "per_key_cycle" {
		if v, ok := p.lastByKey[requestedModel][keyID]; ok && v != "" {
			return v
		}
	}
	if p.strategy == "least_used" {
		best := targets[0]
		bestCount := p.usage[requestedModel][best]
		for _, candidate := range targets[1:] {
			c := p.usage[requestedModel][candidate]
			if c < bestCount || (c == bestCount && candidate < best) {
				best = candidate
				bestCount = c
			}
		}
		p.lastByKey[requestedModel][keyID] = best
		return best
	}

	idx := p.cursor[requestedModel] % len(targets)
	selected := targets[idx]
	p.cursor[requestedModel] = (idx + 1) % len(targets)
	p.lastByKey[requestedModel][keyID] = selected
	return selected
}

func (p *Pool) MarkUsed(alias, actual string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if _, ok := p.usage[alias]; !ok {
		p.usage[alias] = map[string]uint64{}
	}
	p.usage[alias][actual]++
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

func (p *Pool) Aliases() map[string][]string {
	p.mu.Lock()
	defer p.mu.Unlock()
	out := make(map[string][]string, len(p.aliases))
	for k, v := range p.aliases {
		copied := make([]string, len(v))
		copy(copied, v)
		out[k] = copied
	}
	return out
}
