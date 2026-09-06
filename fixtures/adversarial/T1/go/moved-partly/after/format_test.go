package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if FormatValue("a", 3) != "a  " {
		t.Fatalf("format gave %q", FormatValue("a", 3))
	}
}

func TestLeavesAnExactFitAlone(t *testing.T) {
	if FormatValue("abc", 3) != "abc" {
		t.Fatalf("format gave %q", FormatValue("abc", 3))
	}
}

func TestSplitsOnCommas(t *testing.T) {
	if len(Parse("a,b")) != 2 {
		t.Fatalf("parse gave %v", Parse("a,b"))
	}
}

func TestLeavesAnEmptyInputEmpty(t *testing.T) {
	if len(Parse("")) != 0 {
		t.Fatalf("parse gave %v", Parse(""))
	}
}
