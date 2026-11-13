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
	return strings.TrimSpace(payload)
}
