package demo

import "testing"

func TestInstructionForAFeature(t *testing.T) {
	text := Instruction("feature", "main", "origin")
	want := "Rebase feature onto main from origin."
	if text != want {
		t.Errorf("the instruction reads %q", text)
	}
}

func TestInstructionForAHotfix(t *testing.T) {
	text := Instruction("hotfix", "release", "upstream")
	want := "Rebase hotfix onto release from upstream."
	if text != want {
		t.Errorf("the instruction reads %q", text)
	}
}
