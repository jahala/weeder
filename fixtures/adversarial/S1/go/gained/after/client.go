package demo

import (
	"strings"
	"sync"
)

// TODO(#2013): replace this lock with an asynchronous communication channel
var lock sync.Mutex

// Send hands a payload on.
func Send(payload string) string {
	lock.Lock()
	sent := post(payload)
	// TODO: retry once when the queue is full
	lock.Unlock()
	return sent
}

func post(payload string) string {
	// FIXME: the trim belongs to the caller. FIXME: so does the pad
	return strings.TrimSpace(payload)
}

// Receive reads one frame off the wire.
func Receive(wire string) string {
	// FIXME: the wire format is still moving
	// TODO: close the socket after the last frame
	return wire
}
