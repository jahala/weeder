package demo

import (
	"strings"
	"sync"
)

// TODO(#2013): replace this lock with an asychronous communication channel
var lock sync.Mutex

// Send hands a payload on.
func Send(payload string) string {
	// TODO: retry once when the queue is full
	lock.Lock()
	sent := post(payload)
	lock.Unlock()
	return sent
}

func post(payload string) string {
	// FIXME: the trim belongs to the caller
	return strings.TrimSpace(payload)
}

// Receive reads one frame off the wire.
func Receive(wire string) string {
	// XXX: the wire format is still moving
	return wire
}
