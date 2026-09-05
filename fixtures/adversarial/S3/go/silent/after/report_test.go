package demo

import (
	"fmt"
	"testing"
)

func TestSummariseJoinsTheRows(t *testing.T) {
	fmt.Println("what the suite saw", Summarise([]string{"a", "b"}))
	if Summarise([]string{"a", "b"}) != "a, b" {
		t.Fatal("the rows were not joined")
	}
}
