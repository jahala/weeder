package parse

import "testing"

func TestParseReadsANumber(t *testing.T) {
	if got, err := Parse("42"); err != nil || got != 42 {
		t.Errorf("parse gave %d, %v", got, err)
	}
}
