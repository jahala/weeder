package demo

import "testing"

type width struct{ value, want string }

func widths() []width {
	return []width{{"a", "a  "}, {"abcd", "abc"}, {"abc", "abc"}}
}

func TestFormatValue(t *testing.T) {
	for _, tc := range widths() {
		if got := FormatValue(tc.value, 3); got != tc.want {
			t.Errorf("formatting %q gave %q", tc.value, got)
		}
	}
}
