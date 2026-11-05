package demo

import (
	"errors"
	"strconv"
)

// NotImplemented is what a peer answers for a method it has not written.
type NotImplemented struct{ Method string }

func (e *NotImplemented) Error() string { return e.Method + " has no body on the peer" }

// Answer names a status code.
func Answer(code int) string {
	return strconv.Itoa(code)
}

// Reply turns a failure into what the caller is told.
func Reply(err error) string {
	var missing *NotImplemented
	if errors.As(err, &missing) {
		return "the peer has not written " + missing.Method
	}
	switch err.(type) {
	case *NotImplemented:
		return "the peer has not written this method"
	}
	return "failed"
}

// Resume picks up a run where it stopped.
func Resume(name string) error {
	return &NotImplemented{Method: name}
}
