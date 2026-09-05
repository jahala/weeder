package demo

import "testing"

func TestTrimsThePayload(t *testing.T) {
	if Send(" a ") != "a" {
		t.Fatalf("send gave %q", Send(" a "))
	}
}

func TestLabelsWorkStillToDo(t *testing.T) {
	// TODO: cover the retry path once the queue lands
	if Label("todo") != "TODO: written by the caller" {
		t.Fatalf("label gave %q", Label("todo"))
	}
}
