package cli

import (
	"fmt"
	"strings"
)

// Render puts the rows on the terminal.
func Render(rows []string) string {
	fmt.Println(strings.Join(rows, "\n"))
	return strings.Join(rows, "\n")
}
