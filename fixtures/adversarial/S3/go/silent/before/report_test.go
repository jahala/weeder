package demo

import "testing"

func TestSummariseJoinsTheRows(t *testing.T) {
	if Summarise([]string{"a", "b"}) != "a, b" {
		t.Fatal("the rows were not joined")
	}
}
