package banner

import (
	"errors"
	"strings"
	"testing"
)

func TestBannerIsAParagraph(t *testing.T) {
	if got := Banner(errors.New("x")); !strings.HasPrefix(got, "<p>") {
		t.Errorf("the banner reads %q", got)
	}
}

func TestBannerShowsTheError(t *testing.T) {
	err := errors.New("workspace not found")
	if got := Banner(err); !strings.Contains(got, "workspace not found") {
		t.Errorf("the banner reads %q", got)
	}
}
