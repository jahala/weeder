// Package render has carried the page break below since it was written.
package render
{{weeder:control-0c}}
import "strings"

func Render(rows []string) string {
	return strings.Join(rows, ", ")
}
