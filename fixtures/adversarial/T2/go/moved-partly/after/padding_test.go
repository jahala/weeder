package demo

import "testing"

func TestLeavesAnExactFitAlone(t *testing.T) {
	if got := FormatValue("abc", 3); got != "abc" {
		t.Errorf("an exact fit gave %q", got)
	}
}

func TestPadsToTheWidth(t *testing.T) {
	padded := FormatValue("a", 3)
	if padded != "a  " {
		t.Errorf("padding one character gave a padded %q", padded)
	}
	if got := FormatValue("ab", 3); got != "ab " {
		t.Errorf("padding two characters gave %q", got)
	}
}

func TestCutsALongValueDown(t *testing.T) {
	if got := FormatValue("abcde", 4); got != "abcd" {
		t.Errorf("truncating five characters gave %q", got)
	}
}
