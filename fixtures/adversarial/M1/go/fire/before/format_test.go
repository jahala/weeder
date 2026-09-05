package demo

import "testing"

func TestUsesTheDouble(t *testing.T) {
	doubled := NewMockFormatter(t)
	if doubled == nil {
		t.Errorf("the double was not built")
	}
}
