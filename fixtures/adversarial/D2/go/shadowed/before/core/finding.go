package core

import "fmt"

// Render writes a finding as one line.
func Render(rule string, path string) string {
	return fmt.Sprintf("%s %s", rule, path)
}
