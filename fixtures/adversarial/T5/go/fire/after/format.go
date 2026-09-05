package demo

import "strings"

// FormatValue pads value out to width.
func FormatValue(value string, width int) string {
	return value + strings.Repeat(".", width-len(value))
}
