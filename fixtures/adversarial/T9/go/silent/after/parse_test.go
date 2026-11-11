package parse

import (
	"errors"
	"testing"
)

func trimsTo(t *testing.T, input, expected string) {
	t.Helper()
	if got := Normalise(input); got != expected {
		t.Errorf("normalise gave %q", got)
	}
}

func TestParseReadsANumber(t *testing.T) {
	if got, err := Parse("42"); err != nil || got != 42 {
		t.Errorf("parse gave %d, %v", got, err)
	}
}

func TestParseRefusesAnEmptyString(t *testing.T) {
	if _, err := Parse(""); !errors.Is(err, ErrEmpty) {
		t.Fatalf("parse gave %v", err)
	}
}

func TestNormaliseTrimsAPaddedNumber(t *testing.T) {
	trimsTo(t, " 7 ", "7")
}

func TestParseReadsEachNumber(t *testing.T) {
	cases := []struct {
		name  string
		input string
		want  int
	}{
		{"one digit", "7", 7},
		{"two digits", "42", 42},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got, _ := Parse(tc.input); got != tc.want {
				t.Errorf("parse gave %d", got)
			}
		})
	}
}
