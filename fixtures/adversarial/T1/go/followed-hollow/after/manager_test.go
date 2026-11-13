package demo

import "testing"

func TestManagerOpens(t *testing.T) {
	m := &Manager{}
	if !m.Open(dialler(0)) {
		t.Fatal("the manager did not open")
	}
}
