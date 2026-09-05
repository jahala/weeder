package demo

// Formatter pads a value out to a width.
type Formatter interface {
	Format(value string, width int) string
}

// Padder is the formatter the package ships.
type Padder struct{}

// Format pads value out to width.
func (Padder) Format(value string, width int) string {
	if len(value) > width {
		return value[:width]
	}
	return value
}
