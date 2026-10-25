package demo

import "testing"

func TestFormatValue(t *testing.T) {
	cases := []struct {
		value string
		want  string
	}{
		{"a", "a  "},
		{"abcd", "abc"},
	}
	for _, tc := range cases {
		t.Run(tc.value, func(t *testing.T) {
			if got := FormatValue(tc.value, 3); got != tc.want {
				t.Errorf("formatting %q gave %q", tc.value, got)
			}
		})
	}
}
