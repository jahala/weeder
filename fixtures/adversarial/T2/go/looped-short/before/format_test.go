package demo

import "testing"

func TestFormatValue(t *testing.T) {
	if got := FormatValue("a", 3); got != "a  " {
		t.Errorf("formatting %q gave %q", "a", got)
	}
	if got := FormatValue("abcd", 3); got != "abc" {
		t.Errorf("formatting %q gave %q", "abcd", got)
	}
	if got := FormatValue("abc", 3); got != "abc" {
		t.Errorf("formatting %q gave %q", "abc", got)
	}
}
