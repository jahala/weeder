package sprites

// Atlas names every sprite up to a count.
func Atlas(count int) []string {
	names := make([]string, 0, count)
	for index := 0; index < count; index++ {
		names = append(names, Name(index))
	}
	return names
}
