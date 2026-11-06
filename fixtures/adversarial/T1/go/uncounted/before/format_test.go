package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if FormatValue("a", 3) != "a  " {
		t.Fatalf("format gave %q", FormatValue("a", 3))
	}
}

func TestTruncatesPastTheWidth(t *testing.T) {
	if FormatValue("abcd", 3) != "abc" {
		t.Fatalf("format gave %q", FormatValue("abcd", 3))
	}
}

func TestLeavesAValueOfTheWidthAlone(t *testing.T) {
	if FormatValue("abc", 3) != "abc" {
		t.Fatalf("format gave %q", FormatValue("abc", 3))
	}
}
