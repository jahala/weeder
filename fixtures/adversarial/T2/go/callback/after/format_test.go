package demo

import "testing"

type width struct{ value, want string }

func each(cases []width, check func(width)) {
	for _, c := range cases {
		check(c)
	}
}

func TestFormatValue(t *testing.T) {
	each([]width{{"a", "a  "}, {"abcd", "abc"}, {"abc", "abc"}}, func(tc width) {
		if got := FormatValue(tc.value, 3); got != tc.want {
			t.Errorf("formatting %q gave %q", tc.value, got)
		}
	})
}
