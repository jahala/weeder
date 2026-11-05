package demo

import "strings"

// Label tidies a name.
func Label(name string) string {
	return strings.TrimSpace(name)
}

// Translate reads the work a block still holds.
func Translate(todo *string, names []string) string {
	if todo != nil {
		return *todo
	}
	unimplemented := len(names)
	return Label(strings.Repeat("-", unimplemented))
}
