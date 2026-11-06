package demo

import (
	"math"
	"testing"
)

func closeTo(t *testing.T, got float64, want float64, tolerance float64) {
	t.Helper()
	if math.Abs(got-want) > tolerance {
		t.Errorf("got %v, want %v", got, want)
	}
}

func TestRatio(t *testing.T) {
	closeTo(t, Ratio(2, 3), 0.6667, 1e-4)
	closeTo(t, Ratio(1, 3), 0.3333, 1e-4)
}
