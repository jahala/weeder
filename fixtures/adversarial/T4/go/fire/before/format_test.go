package demo

import (
	"math"
	"testing"
	"time"
)

func TestDividesToFourPlaces(t *testing.T) {
	if math.Abs(Ratio(1, 3)-0.3333) > 1e-4 {
		t.Errorf("the ratio drifted further than the suite allows")
	}
}

func TestSettlesBeforeTheDeadline(t *testing.T) {
	if got := Settle(200 * time.Millisecond); got != "done" {
		t.Errorf("settling gave %q", got)
	}
	if got := Total(1, 2); got != 3 {
		t.Errorf("adding gave %d", got)
	}
}
