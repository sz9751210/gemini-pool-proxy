package metrics

import (
	"sort"
	"sync"
	"time"
)

type CallRecord struct {
	At         time.Time `json:"at"`
	KeyID      string    `json:"keyId"`
	MaskedKey  string    `json:"maskedKey"`
	Model      string    `json:"model"`
	StatusCode int       `json:"statusCode"`
	Detail     string    `json:"detail"`
}

type CallsSummary struct {
	Total   uint64 `json:"total"`
	Success uint64 `json:"success"`
	Failure uint64 `json:"failure"`
}

type ModelMetric struct {
	Model   string `json:"model"`
	Total   uint64 `json:"total"`
	Success uint64 `json:"success"`
	Failure uint64 `json:"failure"`
}

type StatusMetric struct {
	StatusCode int    `json:"statusCode"`
	Count      uint64 `json:"count"`
}

type Overview struct {
	Calls24h              CallsSummary   `json:"calls24h"`
	ModelDistribution24h  []ModelMetric  `json:"modelDistribution24h"`
	StatusDistribution24h []StatusMetric `json:"statusDistribution24h"`
	RecentErrors          []CallRecord   `json:"recentErrors"`
}

type Store struct {
	mu      sync.RWMutex
	calls   []CallRecord
	maxLogs int
}

func NewStore(maxLogs int) *Store {
	if maxLogs <= 0 {
		maxLogs = 2000
	}
	return &Store{
		calls:   []CallRecord{},
		maxLogs: maxLogs,
	}
}

func (s *Store) RecordCall(rec CallRecord) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.calls = append(s.calls, rec)
	if len(s.calls) > s.maxLogs {
		s.calls = s.calls[len(s.calls)-s.maxLogs:]
	}
}

func (s *Store) Logs(limit, offset int) []CallRecord {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if limit <= 0 {
		limit = 20
	}
	if offset < 0 {
		offset = 0
	}
	if offset >= len(s.calls) {
		return []CallRecord{}
	}
	end := offset + limit
	if end > len(s.calls) {
		end = len(s.calls)
	}
	out := make([]CallRecord, end-offset)
	copy(out, s.calls[offset:end])
	return out
}

func (s *Store) DashboardOverview(now time.Time) Overview {
	s.mu.RLock()
	defer s.mu.RUnlock()

	cut := now.Add(-24 * time.Hour)
	var summary CallsSummary
	modelMap := map[string]*ModelMetric{}
	statusMap := map[int]uint64{}
	recentErrors := make([]CallRecord, 0, 10)

	for i := len(s.calls) - 1; i >= 0; i-- {
		rec := s.calls[i]
		if rec.StatusCode >= 400 && len(recentErrors) < 10 {
			recentErrors = append(recentErrors, rec)
		}
		if rec.At.Before(cut) {
			continue
		}
		summary.Total++
		if rec.StatusCode >= 200 && rec.StatusCode < 300 {
			summary.Success++
		} else {
			summary.Failure++
		}
		if _, ok := modelMap[rec.Model]; !ok {
			modelMap[rec.Model] = &ModelMetric{Model: rec.Model}
		}
		m := modelMap[rec.Model]
		m.Total++
		if rec.StatusCode >= 200 && rec.StatusCode < 300 {
			m.Success++
		} else {
			m.Failure++
		}
		statusMap[rec.StatusCode]++
	}

	models := make([]ModelMetric, 0, len(modelMap))
	for _, m := range modelMap {
		models = append(models, *m)
	}
	sort.Slice(models, func(i, j int) bool {
		if models[i].Total != models[j].Total {
			return models[i].Total > models[j].Total
		}
		return models[i].Model < models[j].Model
	})

	statuses := make([]StatusMetric, 0, len(statusMap))
	for code, count := range statusMap {
		statuses = append(statuses, StatusMetric{StatusCode: code, Count: count})
	}
	sort.Slice(statuses, func(i, j int) bool {
		return statuses[i].StatusCode < statuses[j].StatusCode
	})

	return Overview{
		Calls24h:              summary,
		ModelDistribution24h:  models,
		StatusDistribution24h: statuses,
		RecentErrors:          recentErrors,
	}
}
