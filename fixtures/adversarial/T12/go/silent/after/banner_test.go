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

func TestBannerWrapsTheError(t *testing.T) {
	err := errors.New("workspace not found")
	if got := Banner(err); got != "<p>workspace not found</p>" {
		t.Errorf("the banner reads %q", got)
	}
}
