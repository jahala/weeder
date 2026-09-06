package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if got := FormatValue("a", 3); got != "a  " {
		t.Errorf("padding one character gave %q", got)
	}
	if got := FormatValue("ab", 3); got != "ab " {
		t.Errorf("padding two characters gave %q", got)
	}
}

func TestTruncatesPastTheWidth(t *testing.T) {
	if got := FormatValue("abcd", 3); got != "abc" {
		t.Errorf("truncating four characters gave %q", got)
	}
	if got := FormatValue("abcde", 4); got != "abcd" {
		t.Errorf("truncating five characters gave %q", got)
	}
}
