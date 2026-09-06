package demo

import "time"

// Ratio divides top by bottom.
func Ratio(top float64, bottom float64) float64 {
	return top / bottom
}

// Total adds two numbers.
func Total(first int, second int) int {
	return first + second
}

// Settle answers once it has waited.
func Settle(waited time.Duration) string {
	return "done"
}
