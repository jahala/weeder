package demo

import "testing"

// dialler answers after the given number of refusals.
func dialler(failures int) func() bool {
	calls := 0
	return func() bool {
		calls++
		return calls > failures
	}
}

func TestClientConnects(t *testing.T) {
	c := &Client{}
	if !c.Connect(func() bool { return true }) {
		t.Fatal("the client did not connect")
	}
}

func TestClientKeepsDialling(t *testing.T) {
	first, answered := retryConnect(dialler(0), 3)
	if !answered {
		t.Fatalf("a server that answers at once was not reached")
	}
	if first != 1 {
		t.Errorf("a server that answers at once took %d tries", first)
	}
	third, answered := retryConnect(dialler(2), 5)
	if !answered {
		t.Fatalf("a server that answers on the third try was not reached")
	}
	if third != 3 {
		t.Errorf("a server that answers on the third try took %d tries", third)
	}
	last, answered := retryConnect(dialler(9), 2)
	if answered {
		t.Fatalf("a server that never answers was reached")
	}
	if last != 2 {
		t.Errorf("a server that never answers took %d tries", last)
	}
}
