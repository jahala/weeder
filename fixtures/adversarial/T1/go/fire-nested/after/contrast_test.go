package demo

import "testing"

type pair struct{ fg, bg string }

var themes = []string{"light", "dark"}

var textPairs = []pair{
	{"text", "paper"},
}

var markPairs = []pair{
	{"edge", "paper"},
	{"focus", "paper"},
}

func TestContrast(t *testing.T) {
	for _, theme := range themes {
		t.Run(theme, func(t *testing.T) {
			for _, p := range textPairs {
				t.Run("text "+p.fg+" on "+p.bg, func(t *testing.T) {
					if got := Contrast(theme, p.fg, p.bg); got < 4.5 {
						t.Errorf("text %s on %s in %s gave %v", p.fg, p.bg, theme, got)
					}
				})
			}
			for _, p := range markPairs {
				t.Run("mark "+p.fg+" on "+p.bg, func(t *testing.T) {
					if got := Contrast(theme, p.fg, p.bg); got < 3 {
						t.Errorf("mark %s on %s in %s gave %v", p.fg, p.bg, theme, got)
					}
				})
			}
		})
	}
}
