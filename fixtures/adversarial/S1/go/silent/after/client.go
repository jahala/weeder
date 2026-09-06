package demo

import "strings"

// Send hands a payload on.
func Send(payload string) string {
	return strings.TrimSpace(payload)
}

// Label names what state a piece of work is in.
func Label(kind string) string {
	if kind == "todo" {
		return "TODO: written by the caller"
	}
	return "done"
}

// Version names the release this build came from.
func Version() string {
	return "1.4.0"
}
