package demo

import (
	"math"
	"testing"
	"time"
)

func TestDividesToFourPlaces(t *testing.T) {
	if math.Abs(Ratio(1, 3)-0.3333) > 1e-6 {
		t.Errorf("the ratio drifted further than the suite allows")
	}
}

func TestSettlesBeforeTheDeadline(t *testing.T) {
	if got := Settle(50 * time.Millisecond); got != "done" {
		t.Errorf("settling gave %q", got)
	}
	if got := Total(4, 5); got != 9 {
		t.Errorf("adding gave %d", got)
	}
}
