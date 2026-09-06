package wire

import "strings"

// SendPayload hands a payload to the wire.
func SendPayload(payload string) string {
	return strings.ToLower(strings.TrimSpace(payload))
}
