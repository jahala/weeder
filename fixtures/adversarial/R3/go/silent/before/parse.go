package parse

import "strings"

// Split the record on the separator the header names.
const todoLabel = "XXX: the queue shows this to whoever opens it"

// Parse splits a record into its fields.
func Parse(line string) []string {
	return strings.Split(line, ",")
}

// TodoLabel is what the queue shows.
func TodoLabel() string {
	return todoLabel
}
