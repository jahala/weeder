package service

import "testing"

func TestEndpointBuildsAURL(t *testing.T) {
	if got := Endpoint("example.org"); got != "https://example.org/" {
		t.Errorf("endpoint gave %q", got)
	}
}

func TestServiceName(t *testing.T) {
	if Service != "build.example.app" {
		t.Errorf("service is %q", Service)
	}
}
