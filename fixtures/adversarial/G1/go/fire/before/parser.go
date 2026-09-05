package parser

import "strings"

// Parse splits an input row into its fields.
func Parse(input string) []string {
	return strings.Split(input, ",")
}
