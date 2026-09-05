package demo

// Send hands a payload on.
func Send(payload string) error {
	if err := post(payload); err != nil {
		return nil
	}
	return nil
}

// Receive takes a payload off the wire.
func Receive() string {
	value, err := read()
	_ = err
	return value
}
