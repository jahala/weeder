// Package casting picks the worker for a node {{weeder:control-00}} by provider first, then by cost.
package casting

import (
	"fmt"
	"strings"
)

func Cast(provider string, cost int) string {
	name := strings.TrimSpace(provider)
	return fmt.Sprintf("%s at %d {{weeder:control-00}} the cheapest that can do it", name, cost)
}
