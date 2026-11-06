package demo

import "testing"

func TestInstructionNamesTheBranchTheBaseAndTheRemote(t *testing.T) {
	want := "Rebase feature onto main from origin."
	text := Instruction("feature", "main", "origin")
	if text != want {
		t.Errorf("the instruction reads %q", text)
	}
	again := Instruction("feature", "main", "origin")
	if again != want {
		t.Errorf("the instruction asked again reads %q", again)
	}
}
