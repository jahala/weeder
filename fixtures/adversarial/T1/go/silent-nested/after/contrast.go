package demo

var ratios = map[string]float64{
	"light text paper":  12.6,
	"light muted paper": 5.1,
	"light edge paper":  3.4,
	"light focus paper": 4.2,
	"dark text paper":   13.1,
	"dark muted paper":  5.6,
	"dark edge paper":   3.2,
	"dark focus paper":  4.8,
}

// Contrast is the ratio of fg over bg in a theme, or 1 where none is known.
func Contrast(theme, fg, bg string) float64 {
	if ratio, ok := ratios[theme+" "+fg+" "+bg]; ok {
		return ratio
	}
	return 1
}
