package service

import "testing"

func TestEndpointBuildsAURL(t *testing.T) {
	if got := Endpoint("example.org"); got != "https://example.org/" {
		t.Errorf("endpoint gave %q", got)
	}
}
