package greet

import "testing"

func TestGreet(t *testing.T) {
	if Greet("weed") != "hello weed" {
		t.Fatalf("greeting was %q", Greet("weed"))
	}
}
