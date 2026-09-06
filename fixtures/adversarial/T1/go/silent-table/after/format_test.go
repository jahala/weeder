package demo

import "testing"

func TestFormatValue(t *testing.T) {
	cases := []struct {
		name  string
		value string
		want  string
	}{
		{"pads to the width", "a", "a  "},
		{"truncates past the width", "abcd", "abc"},
		{"leaves a value of the width alone", "abc", "abc"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if FormatValue(tc.value, 3) != tc.want {
				t.Fatalf("format gave %q", FormatValue(tc.value, 3))
			}
		})
	}
}
