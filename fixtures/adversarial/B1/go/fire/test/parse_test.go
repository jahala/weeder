package records

import (
	"reflect"
	"testing"
)

func TestSplitsOnCommas(t *testing.T) {
	if !reflect.DeepEqual(Parse("a,b"), []string{"a", "b"}) {
		t.Fatal("a comma should end a field")
	}
}

func TestKeepsASingleFieldWhole(t *testing.T) {
	if !reflect.DeepEqual(Parse("a"), []string{"a"}) {
		t.Fatal("a row with no separator is one field")
	}
}
