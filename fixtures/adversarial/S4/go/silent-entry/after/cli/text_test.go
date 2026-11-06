package cli

import "testing"

func TestSlugify(t *testing.T) {
	if Slugify(" A b ") != "a-b" {
		t.Fatal("slugify should join the words with a dash")
	}
}
