package demo

import (
	"os"
	"testing"
)

func TestPadsToTheWidth(t *testing.T) {
	want, err := os.ReadFile("testdata/format.golden")
	if err != nil {
		t.Fatalf("reading the golden file: %%v", err)
	}
	if got := FormatValue("a", 4); got != string(want) {
		t.Errorf("padding gave %%q", got)
	}
}
