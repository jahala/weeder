package demo

import "testing"

func TestFormatValue(t *testing.T) {
	for _, tc := range []struct{ value, want string }{
		{"a", "a  "},
		{"abcd", "abc"},
		{"abc", "abc"},
	} {
		if got := FormatValue(tc.value, 3); got != tc.want {
			t.Errorf("formatting %q gave %q", tc.value, got)
		}
	}
}
