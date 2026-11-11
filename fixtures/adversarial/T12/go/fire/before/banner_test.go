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
