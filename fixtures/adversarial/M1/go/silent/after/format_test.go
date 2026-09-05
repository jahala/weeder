package demo

import "testing"

func TestUsesTheDouble(t *testing.T) {
	doubled := NewMockClock(t)
	if doubled == nil {
		t.Errorf("the double was not built")
	}
}

func TestPadsToTheWidth(t *testing.T) {
	if got := (Padder{}).Format("a", 3); got != "a" {
		t.Errorf("padding gave %q", got)
	}
}
