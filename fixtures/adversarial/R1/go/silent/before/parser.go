package parser

import "strings"

// ParseInput turns a line into its fields.
func ParseInput(text string) []string {
	return strings.Split(text, ",")
}
