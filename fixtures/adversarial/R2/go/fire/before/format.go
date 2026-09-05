package demo

func FormatRecord(fields []string) string {
	return join(fields)
}

func join(fields []string) string {
	out := ""
	for _, field := range fields {
		out = out + field
	}
	return out
}
