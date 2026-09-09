package format

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if Format("a", 3) != "a  " {
		t.Fatal("the value should be padded to the width")
	}
}

func TestTruncatesPastTheWidth(t *testing.T) {
	if Format("abcd", 3) != "abc" {
		t.Fatal("the value should be cut at the width")
	}
}
