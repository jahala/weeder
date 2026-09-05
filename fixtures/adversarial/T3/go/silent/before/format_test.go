package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if FormatValue("a", 3) != "a  " {
		t.Fatalf("format gave %q", FormatValue("a", 3))
	}
}
