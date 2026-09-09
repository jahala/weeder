//go:build slow

package format

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if Format("a", 3) != "a  " {
		t.Fatal("the value should be padded to the width")
	}
}
