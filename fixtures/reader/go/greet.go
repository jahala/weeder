package greet

import "strings"

// Greet returns a greeting for name.
func Greet(name string) string {
	return strings.Join([]string{"hello", name}, " ")
}
