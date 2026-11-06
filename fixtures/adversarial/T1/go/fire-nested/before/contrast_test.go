package demo

import "testing"

func TestTextOnPaperReadsInLight(t *testing.T) {
	if got := Contrast("light", "text", "paper"); got < 4.5 {
		t.Errorf("text on paper in light gave %v", got)
	}
}

func TestMutedOnPaperReadsInLight(t *testing.T) {
	if got := Contrast("light", "muted", "paper"); got < 4.5 {
		t.Errorf("muted on paper in light gave %v", got)
	}
}

func TestEdgeOnPaperShowsInLight(t *testing.T) {
	if got := Contrast("light", "edge", "paper"); got < 3 {
		t.Errorf("edge on paper in light gave %v", got)
	}
}

func TestFocusOnPaperShowsInLight(t *testing.T) {
	if got := Contrast("light", "focus", "paper"); got < 3 {
		t.Errorf("focus on paper in light gave %v", got)
	}
}

func TestTextOnPaperReadsInDark(t *testing.T) {
	if got := Contrast("dark", "text", "paper"); got < 4.5 {
		t.Errorf("text on paper in dark gave %v", got)
	}
}

func TestMutedOnPaperReadsInDark(t *testing.T) {
	if got := Contrast("dark", "muted", "paper"); got < 4.5 {
		t.Errorf("muted on paper in dark gave %v", got)
	}
}

func TestEdgeOnPaperShowsInDark(t *testing.T) {
	if got := Contrast("dark", "edge", "paper"); got < 3 {
		t.Errorf("edge on paper in dark gave %v", got)
	}
}

func TestFocusOnPaperShowsInDark(t *testing.T) {
	if got := Contrast("dark", "focus", "paper"); got < 3 {
		t.Errorf("focus on paper in dark gave %v", got)
	}
}
