package demo

import "testing"

func TestSplitsOnCommas(t *testing.T) {
	if len(Parse("a,b")) != 2 {
		t.Fatalf("parse gave %v", Parse("a,b"))
	}
}

func TestLeavesAnEmptyInputEmpty(t *testing.T) {
	if len(Parse("")) != 0 {
		t.Fatalf("parse gave %v", Parse(""))
	}
}
