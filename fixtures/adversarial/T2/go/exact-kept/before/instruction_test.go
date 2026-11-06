package demo

import (
	"strings"
	"testing"
)

func TestInstructionNamesTheBranchTheBaseAndTheRemote(t *testing.T) {
	text := Instruction(
		"feature", "main", "origin",
	)
	want := strings.Join([]string{
		"Rebase feature onto main from origin.",
	}, "")
	if text != want {
		t.Errorf("the instruction reads %q", text)
	}
	text = Instruction(
		"hotfix", "release", "upstream",
	)
	want = strings.Join([]string{
		"Rebase hotfix onto release from upstream.",
	}, "")
	if text != want {
		t.Errorf("the instruction reads %q", text)
	}
}
