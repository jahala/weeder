package demo

import "testing"

func TestUsesTheDouble(t *testing.T) {
	doubled := NewMockClock(t)
	if doubled == nil {
		t.Errorf("the double was not built")
	}
}
