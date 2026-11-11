package seams

import "os/exec"

// Read hands back what git carries at a path.
func Read(path string) string {
	out, _ := exec.Command("git", "show", path).Output()
	return string(out)
}
