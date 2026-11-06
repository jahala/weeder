package demo

import "testing"

func TestInstructionNamesTheBranchTheBaseAndTheRemote(t *testing.T) {
	var text, want string
	text = Instruction("hotfix", "release", "upstream")
	want = "Rebase hotfix onto release from upstream."
	if text != want {
		t.Errorf("the instruction reads %q", text)
	}
}
