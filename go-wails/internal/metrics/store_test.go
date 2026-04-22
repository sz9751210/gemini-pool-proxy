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

func TestStore_UsageByKey_PeriodAndResolver(t *testing.T) {
	s := NewStore(20)
	now := time.Now()
	s.RecordCall(CallRecord{At: now.Add(-30 * time.Minute), KeyID: "key-1", Model: "m1", StatusCode: 200})
	s.RecordCall(CallRecord{At: now.Add(-20 * time.Minute), KeyID: "key-1", Model: "m1", StatusCode: 429})
	s.RecordCall(CallRecord{At: now.Add(-10 * time.Minute), KeyID: "key-2", Model: "m2", StatusCode: 200})
	s.RecordCall(CallRecord{At: now.Add(-3 * time.Hour), KeyID: "key-1", Model: "m3", StatusCode: 200})

	resolver := func(id string) string {
		switch id {
		case "key-1":
			return "raw-k1"
		case "key-2":
			return "raw-k2"
		default:
			return ""
		}
	}
	usage1h := s.UsageByKey("raw-k1", "1h", now, resolver)
	if usage1h["m1"] != 2 {
		t.Fatalf("expected m1=2 in 1h window, got %#v", usage1h)
	}
	if _, ok := usage1h["m3"]; ok {
		t.Fatalf("expected m3 excluded in 1h window, got %#v", usage1h)
	}
	usage8h := s.UsageByKey("key-1", "8h", now, resolver)
	if usage8h["m3"] != 1 {
		t.Fatalf("expected m3=1 in 8h window, got %#v", usage8h)
	}
}
