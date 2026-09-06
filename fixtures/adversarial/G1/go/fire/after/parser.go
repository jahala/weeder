package parser

import "strings"

// Parse splits an input row into its fields.
func Parse(input string) []string {
{{weed:ours}} HEAD
	return strings.Split(input, ",")
{{weed:separator}}
	return strings.Split(input, ";")
{{weed:theirs}} feature/split-on-semicolons
}
