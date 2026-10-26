package demo

import (
	"os"
	"strings"
	"testing"
)

func TestKeepsAValueAlreadyAtTheWidth(t *testing.T) {
	if got := FormatValue("abcd", 4); got != "abcd" {
		t.Errorf("a full value changed: got " + got)
	}
}

func TestPadsAShortValueWithDots(t *testing.T) {
	want, err := os.ReadFile("testdata/padded.txt")
	if err != nil {
		t.Fatal(err)
	}
	if got := FormatValue("a", 4); got != strings.TrimSpace(string(want)) {
		t.Errorf("padding gave " + got)
	}
}
