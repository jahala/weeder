package text

import "strings"

func Trim(value string) string {
	return strings.TrimSpace(value)
}

func Slugify(value string) string {
	return strings.Join(strings.Fields(strings.ToLower(Trim(value))), "-")
}
