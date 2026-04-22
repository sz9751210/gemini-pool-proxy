package metrics

import (
	"testing"
	"time"
)

func TestStore_RecordAndOverview(t *testing.T) {
	s := NewStore(2000)
	now := time.Now()
	s.RecordCall(CallRecord{At: now, KeyID: "key-1", Model: "m1", StatusCode: 200})
	s.RecordCall(CallRecord{At: now, KeyID: "key-1", Model: "m1", StatusCode: 429})
	overview := s.DashboardOverview(now)
	if overview.Calls24h.Total != 2 || overview.Calls24h.Failure != 1 {
		t.Fatalf("unexpected overview: %#v", overview)
	}
}

func TestStore_LogsPagination(t *testing.T) {
	s := NewStore(10)
	now := time.Now()
	for i := 0; i < 3; i++ {
		s.RecordCall(CallRecord{At: now, KeyID: "k", Model: "m", StatusCode: 200 + i})
	}
	page := s.Logs(2, 1)
	if len(page) != 2 {
		t.Fatalf("expected 2 records, got %d", len(page))
	}
	if page[0].StatusCode != 201 {
		t.Fatalf("expected status 201, got %d", page[0].StatusCode)
	}
}
