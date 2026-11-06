package demo

import (
	"errors"
	"testing"
)

func TestParse(t *testing.T) {
	if _, err := Parse(""); err == nil {
		t.Error("an empty input parsed")
	}
	if _, err := Parse(";;"); !errors.Is(err, ErrSeparator) {
		t.Errorf("a stray separator gave %v", err)
	}
}
