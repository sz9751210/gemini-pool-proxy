package admin

import (
	"crypto/rand"
	"encoding/hex"
	"sync"
)

type SessionStore struct {
	mu    sync.RWMutex
	items map[string]struct{}
}

func NewSessionStore() *SessionStore {
	return &SessionStore{items: map[string]struct{}{}}
}

func (s *SessionStore) Create() string {
	buf := make([]byte, 16)
	_, _ = rand.Read(buf)
	id := hex.EncodeToString(buf)
	s.mu.Lock()
	s.items[id] = struct{}{}
	s.mu.Unlock()
	return id
}

func (s *SessionStore) Has(id string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	_, ok := s.items[id]
	return ok
}

func (s *SessionStore) Delete(id string) {
	s.mu.Lock()
	delete(s.items, id)
	s.mu.Unlock()
}
