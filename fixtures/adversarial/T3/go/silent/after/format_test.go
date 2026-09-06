package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if FormatValue("a", 3) != "a  " {
		t.Fatalf("format gave %q", FormatValue("a", 3))
	}
}

func TestTruncatesPastTheWidth(t *testing.T) {
	// t.Skip() came off this case when the padding stopped rounding.
	if FormatValue("abcd", 3) != "abc" {
		t.Fatalf("format gave %q", FormatValue("abcd", 3))
	}
}

func TestNamesTheMarkerAReviewerGrepsFor(t *testing.T) {
	if marker() != "t.SkipNow()" {
		t.Fatalf("marker was %q", marker())
	}
}
