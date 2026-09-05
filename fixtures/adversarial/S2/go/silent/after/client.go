package demo

import "fmt"

// Send hands a payload on.
func Send(payload string) error {
	if err := post(payload); err != nil {
		return fmt.Errorf("send %q: %w", payload, err)
	}
	return nil
}

// Receive takes a payload off the wire.
func Receive() (string, error) {
	value, err := read()
	if err != nil {
		return "", fmt.Errorf("receive: %w", err)
	}
	return value, nil
}
