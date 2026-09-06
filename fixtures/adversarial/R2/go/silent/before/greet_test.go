package demo

import "testing"

func TestGreet(t *testing.T) {
	if Greet("world") == "" {
		t.Fatal("the greeting is empty")
	}
}
