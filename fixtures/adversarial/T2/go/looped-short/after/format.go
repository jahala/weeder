package demo

import "strings"

// FormatValue pads value out to width, or cuts it down to width.
func FormatValue(value string, width int) string {
	if len(value) > width {
		return value[:width]
	}
	return value + strings.Repeat(" ", width-len(value))
}
