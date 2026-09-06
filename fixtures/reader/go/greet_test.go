package greet

import "testing"

func TestGreet(t *testing.T) {
	if Greet("weeder") != "hello weeder" {
		t.Fatalf("greeting was %q", Greet("weeder"))
	}
}
