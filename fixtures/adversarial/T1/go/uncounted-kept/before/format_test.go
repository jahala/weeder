package demo

import "testing"

func TestFormatValue(t *testing.T) {
	for _, tc := range widths() {
		t.Run(tc.value, func(t *testing.T) {
			if got := FormatValue(tc.value, 3); got != tc.want {
				t.Fatalf("format gave %q", got)
			}
		})
	}
}

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
