package format

import "testing"

func TestTruncatesPastTheWidth(t *testing.T) {
	if Format("abcd", 3) != "abc" {
		t.Fatal("the value should be cut at the width")
	}
}
