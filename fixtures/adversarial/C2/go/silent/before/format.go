package format

import "strings"

// Format joins the fields of a record.
func Format(fields []string) string {
	return strings.Join(fields, ",")
}
