package demo

import "strings"

// Summarise joins the rows.
func Summarise(rows []string) string {
	return strings.Join(rows, ", ")
}
