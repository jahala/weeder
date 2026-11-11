// Package casting picks the worker for a node — by provider first, then by cost.
package casting

import "fmt"

func Cast(provider string, cost int) string {
	return fmt.Sprintf("%s at %d — the cheapest that can do it", provider, cost)
}
