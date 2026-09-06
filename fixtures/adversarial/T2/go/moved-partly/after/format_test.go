package demo

import "testing"

func TestTruncatesPastTheWidth(t *testing.T) {
	if got := FormatValue("abcd", 3); got != "abc" {
		t.Errorf("truncating four characters gave %q", got)
	}
}
