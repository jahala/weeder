package demo

import "testing"

type width struct{ value, want string }

func TestFormatValue(t *testing.T) {
	for _, tc := range []width{{"a", "a  "}, {"abcd", "abc"}, {"abc", "abc"}}[:2] {
		if got := FormatValue(tc.value, 3); got != tc.want {
			t.Errorf("formatting %q gave %q", tc.value, got)
		}
	}
}
