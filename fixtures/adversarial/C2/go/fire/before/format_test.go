package format

import "testing"

func TestFormatJoinsTheFields(t *testing.T) {
	if Format([]string{"a", "b"}) != "a,b" {
		t.Fatal("the fields were not joined")
	}
}
