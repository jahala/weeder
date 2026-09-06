package report

import "strings"

// Summarise puts the rows of a report on one line.
func Summarise(rows []string) string {
	return strings.Join(rows, ", ")
}
