package demo

import "testing"

func TestPadsToTheWidth(t *testing.T) {
	if got := (Padder{}).Format("a", 3); got != "a" {
		t.Errorf("padding gave %q", got)
	}
}
