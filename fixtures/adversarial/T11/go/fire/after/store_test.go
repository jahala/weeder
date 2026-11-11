package store

import "testing"

type FakeStore struct {
	Items []string
}

func (fake *FakeStore) Count() int {
	return len(fake.Items)
}

func TestSummariseAnEmptyStore(t *testing.T) {
	if got := Summarise(&FakeStore{}); got.Total != 0 {
		t.Errorf("an empty store summed to %d", got.Total)
	}
}

func TestStoreCountsItems(t *testing.T) {
	fake := &FakeStore{Items: []string{"a", "b"}}
	if got := fake.Count(); got != 2 {
		t.Errorf("the store counted %d", got)
	}
}
