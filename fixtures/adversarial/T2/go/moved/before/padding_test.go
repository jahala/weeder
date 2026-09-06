package demo

import "testing"

func TestLeavesAnExactFitAlone(t *testing.T) {
	if got := FormatValue("abc", 3); got != "abc" {
		t.Errorf("an exact fit gave %q", got)
	}
}
