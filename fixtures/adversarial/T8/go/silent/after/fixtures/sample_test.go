//go:build slow

package fixtures

import "testing"

func TestSampleRowIsWideEnough(t *testing.T) {
	if len(SampleRow) < 3 {
		t.Fatal("the sample row is what the formatter is read against")
	}
}
