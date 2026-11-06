package core

import (
	"fmt"

	"example.com/demo/faces"
	"example.com/demo/seams"
)

// Render writes a finding as one line.
func Render(rule string, path string) string {
	return fmt.Sprintf("%s %s %s %s", rule, path, seams.Read(path), faces.Name(path))
}
