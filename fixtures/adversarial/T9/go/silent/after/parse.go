package parse

import (
	"errors"
	"strconv"
	"strings"
)

var ErrEmpty = errors.New("empty input")

func Parse(input string) (int, error) {
	trimmed := strings.TrimSpace(input)
	if trimmed == "" {
		return 0, ErrEmpty
	}
	return strconv.Atoi(trimmed)
}

func Normalise(input string) string {
	return strings.TrimSpace(input)
}
