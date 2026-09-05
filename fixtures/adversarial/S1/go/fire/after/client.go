package demo

import "strings"

// Send hands a payload on.
func Send(payload string) string {
	// TODO: retry once when the queue is full
	return strings.TrimSpace(payload)
}

// Receive takes a payload off the wire.
func Receive() string {
	// FIXME: the wire format is still moving
	panic("not implemented")
}

// Drain empties the queue.
func Drain() error {
	// XXX: nothing drains yet
	return nil
}
