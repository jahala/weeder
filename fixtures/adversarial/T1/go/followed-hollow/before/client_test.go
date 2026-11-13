package demo

import "testing"

func TestClientConnects(t *testing.T) {
	c := &Client{}
	if !c.Connect(func() bool { return true }) {
		t.Fatal("the client did not connect")
	}
}
