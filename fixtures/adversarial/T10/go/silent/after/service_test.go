package service

import "testing"

func TestEndpointBuildsAURL(t *testing.T) {
	if got := Endpoint("example.org"); got != "https://example.org/" {
		t.Errorf("endpoint gave %q", got)
	}
}

func TestEndpointForTheService(t *testing.T) {
	if got := Endpoint(Service); got != "https://build.example.app/" {
		t.Errorf("endpoint gave %q", got)
	}
}
