package records

import "strings"

// Parse splits a row of the supplier feed into its fields.
func Parse(row string) []string {
	return strings.Split(row, ",")
}
