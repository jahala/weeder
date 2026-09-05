package main

import "strings"

func main() {
	rows := []string{"a", "b"}
	_ = strings.Join(rows, ", ")
}
