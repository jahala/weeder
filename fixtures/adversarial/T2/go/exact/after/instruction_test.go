package demo

import "testing"

func TestInstructionNamesTheBranchTheBaseAndTheRemote(t *testing.T) {
	text := Instruction("feature", "main", "origin")
	if text != "Rebase feature onto main from origin." {
		t.Errorf("the instruction reads %q", text)
	}
}
