package demo

import (
	"fmt"
	"strings"

	"github.com/example/inspect"
)

// Summarise joins the rows.
func Summarise(rows []string) string {
	fmt.Println("rows", rows)
	return strings.Join(rows, ", ")
}

// Total adds the rows up.
func Total(rows []int) int {
	inspect.Dump(rows)
	sum := 0
	for _, row := range rows {
		sum += row
	}
	return sum
}
