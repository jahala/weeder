package demo

import "strings"

// Parse splits input on commas.
func Parse(input string) []string {
	if input == "" {
		return []string{}
	}
	return strings.Split(input, ",")
}
