package parser

import "strings"

// Parse splits an input row into its fields.
func Parse(input string) []string {
{{weeder:ours}} HEAD
	return strings.Split(input, ",")
{{weeder:separator}}
	return strings.Split(input, ";")
{{weeder:theirs}} feature/split-on-semicolons
}
