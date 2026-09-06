package cli

import "strings"

// Render puts the rows on the terminal.
func Render(rows []string) string {
	return strings.Join(rows, "\n")
}
