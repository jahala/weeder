package demo

import (
	"errors"
	"testing"
)

func TestRefusesAnEmptyInput(t *testing.T) {
	if _, err := Parse(""); !errors.Is(err, ErrEmpty) {
		t.Errorf("parsing an empty input let it through")
	}
}

func TestRefusesAStraySeparator(t *testing.T) {
	if _, err := Parse(";;"); err.Error() != "stray separator" {
		t.Errorf("parsing a stray separator let it through")
	}
}

func TestRefusesATrailingSeparator(t *testing.T) {
	if _, err := Parse("a;"); !errors.Is(err, ErrTrailing) {
		t.Errorf("parsing a trailing separator let it through")
	}
}
