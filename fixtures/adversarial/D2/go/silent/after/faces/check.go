package faces

import (
	"example.com/demo/core"
	"example.com/demo/seams"
)

// Check judges one path.
func Check(path string) string {
	return core.Render("T1", seams.Read(path))
}
