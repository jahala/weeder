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
	if _, answered := retryConnect(dialler(2), 5); !answered {
		t.Fatalf("a server that answers on the third try was not reached")
	}
}
