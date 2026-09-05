package demo

import "strings"

// Summarise joins the rows.
func Summarise(rows []string) string {
	return strings.Join(rows, ", ")
}

// Total adds the rows up.
func Total(rows []int) int {
	sum := 0
	for _, row := range rows {
		sum += row
	}
	return sum
}
