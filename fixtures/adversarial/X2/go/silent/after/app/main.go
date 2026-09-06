package app

import (
	"strings"

	"demo/report"
	"demo/wire"
)

// Run sends the rows and reports what came back.
func Run(rows []string) string {
	return report.Summarise([]string{wire.SendPayload(strings.Join(rows, ","))})
}
