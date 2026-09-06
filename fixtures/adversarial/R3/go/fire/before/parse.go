package parse

import "strings"

// XXX: split the record on the separator the header names
func Parse(line string) []string {
	return strings.Split(line, ",")
}
