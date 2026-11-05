package demo

import "strings"

// Label tidies a name.
func Label(name string) string {
	return strings.TrimSpace(name)
}
