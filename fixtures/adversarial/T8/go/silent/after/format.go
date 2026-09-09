package format

import "strings"

func Format(value string, width int) string {
	if len(value) > width {
		return value[:width]
	}
	return value + strings.Repeat(" ", width-len(value))
}
