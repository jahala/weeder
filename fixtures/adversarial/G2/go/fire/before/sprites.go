package sprites

import "fmt"

// Name names the sprite at an index.
func Name(index int) string {
	return fmt.Sprintf("sprite-%d", index)
}
