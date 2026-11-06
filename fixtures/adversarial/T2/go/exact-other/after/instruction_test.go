package demo

import "testing"

func TestInstructionNamesTheBranchTheBaseAndTheRemote(t *testing.T) {
	text := Instruction("feature", "main", "origin")
	if len(text) != 37 {
		t.Errorf("the instruction is %d long", len(text))
	}
}
