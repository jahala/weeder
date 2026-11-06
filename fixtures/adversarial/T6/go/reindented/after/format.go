package demo

import (
	"errors"
	"strings"
)

// ErrEmpty is the failure a caller gets for an input with nothing in it.
var ErrEmpty = errors.New("input is empty")

// ErrTrailing is the failure a caller gets for an input that ends on a separator.
var ErrTrailing = errors.New("trailing separator")

// Parse splits input on semicolons.
func Parse(input string) ([]string, error) {
	if input == "" {
		return nil, ErrEmpty
	}
	if strings.Contains(input, ";;") {
		return nil, errors.New("stray separator")
	}
	if strings.HasSuffix(input, ";") {
		return nil, ErrTrailing
	}
	return strings.Split(input, ";"), nil
}
