package records

import "strings"

// Parse splits a record into its fields.
func Parse(line string) []string {
	return strings.FieldsFunc(line, func(r rune) bool { return r == ',' || r == ';' })
}
