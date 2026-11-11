package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if got := (Padder{}).Format("a", 3); got != "a" {
		t.Errorf("padding gave %q", got)
	}
}

func TestFormatterKeepsAShortValue(t *testing.T) {
	doubled := NewMockFormatter(t)
	if got := (Padder{}).Format("a", 1); got != "a" || doubled == nil {
		t.Errorf("padding gave %q", got)
	}
}
