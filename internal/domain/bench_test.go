package domain

import (
	"fmt"
	"testing"

	"github.com/AmaneKai/hopes/internal/gitsync"
)

func benchState(n int) *AppState {
	items := make([]Item, n)
	for i := range items {
		items[i] = NewItem(fmt.Sprintf("Task number %d Mixed Case", i), fmt.Sprintf("ws%d", i%8))
		items[i].Description = "some longer description text for matching purposes"
	}
	s := NewAppState(items, gitsyncIdle())
	s.Workspace = "ws3"
	return s
}

func BenchmarkRefreshFilterSortTitle(b *testing.B) {
	s := benchState(20000)
	s.HasFilter, s.Filter, s.SortMode = true, "case", SortTitle
	b.ReportAllocs()
	for b.Loop() {
		s.RefreshCache()
	}
}

func BenchmarkRefreshPlain(b *testing.B) {
	s := benchState(20000)
	b.ReportAllocs()
	for b.Loop() {
		s.RefreshCache()
	}
}

func gitsyncIdle() gitsync.Status { return gitsync.Status{} }
