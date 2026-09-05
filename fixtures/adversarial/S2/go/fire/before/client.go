package demo

// Send hands a payload on.
func Send(payload string) error {
	return post(payload)
}

// Receive takes a payload off the wire.
func Receive() (string, error) {
	return read()
}
