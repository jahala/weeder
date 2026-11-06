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
