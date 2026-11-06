package demo

import "fmt"

// Instruction is what to tell an agent to do with its branch.
func Instruction(branch, base, remote string) string {
	return fmt.Sprintf("Rebase %s onto %s from %s.", branch, base, remote)
}
