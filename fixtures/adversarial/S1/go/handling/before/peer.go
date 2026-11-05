package demo

import "strconv"

// Answer names a status code.
func Answer(code int) string {
	return strconv.Itoa(code)
}
