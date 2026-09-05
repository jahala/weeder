package banner

import "fmt"

const rule = "======="

// Banner writes a title over a rule.
func Banner(title string) string {
	return fmt.Sprintf("%s\n%s", title, rule)
}
