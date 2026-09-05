package faces

import "example.com/demo/core"

// Check judges one path.
func Check(path string) string {
	return core.Render("T1", path)
}
