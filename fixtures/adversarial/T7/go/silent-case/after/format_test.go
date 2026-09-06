package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if got := FormatValue("a", 3); got != "a  " {
		t.Errorf("padding gave %q", got)
	}
}

func TestTruncatesPastTheWidth(t *testing.T) {
	if got := FormatValue("abcd", 3); got != "abc" {
		t.Errorf("truncating gave %q", got)
	}
}
