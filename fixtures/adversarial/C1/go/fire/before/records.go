package records

import "strings"

// Parse splits a record into its fields.
func Parse(line string) []string {
	return strings.Split(line, ",")
}
