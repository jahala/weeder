package demo

import "strings"

// Pad widens a string.
func Pad(text string, width int) string {
	return strings.Repeat(" ", width-len(text)) + text
}
