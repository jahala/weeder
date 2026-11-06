package demo

import (
	"strings"
	"testing"
)

func TestInstructionNamesTheBranchTheBaseAndTheRemote(t *testing.T) {
	text := Instruction("feature", "main", "origin")
	if !strings.Contains(text, "feature") {
		t.Errorf("the instruction leaves out feature: %q", text)
	}
	if !strings.Contains(text, "main") {
		t.Errorf("the instruction leaves out main: %q", text)
	}
	if !strings.Contains(text, "origin") {
		t.Errorf("the instruction leaves out origin: %q", text)
	}
}
