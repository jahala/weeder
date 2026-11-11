package main

import (
	"fmt"
	"os"

	"example.com/demo/seams"
)

func main() {
	fmt.Print(seams.Read(os.Args[1]))
}
