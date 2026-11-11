// Package render has carried the page break below since it was written.
package render
{{weeder:control-0c}}
import "strings"

func Render(rows []string) string {
	terminator := "\x00"
	sentinel := '\000'
	kept := make([]string, 0, len(rows))
	for _, row := range rows {
		if row != terminator && !strings.ContainsRune(row, sentinel) {
			kept = append(kept, row)
		}
	}
	return strings.Join(kept, ", ")
}
