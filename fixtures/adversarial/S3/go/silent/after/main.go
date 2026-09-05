package main

import (
	"fmt"
	"strings"
)

func main() {
	rows := []string{"a", "b"}
	fmt.Println(strings.Join(rows, ", "))
}
