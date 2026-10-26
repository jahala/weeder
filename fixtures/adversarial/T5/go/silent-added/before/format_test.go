package demo

import "testing"

func TestKeepsAValueAlreadyAtTheWidth(t *testing.T) {
	if got := FormatValue("abcd", 4); got != "abcd" {
		t.Errorf("a full value changed: got " + got)
	}
}
