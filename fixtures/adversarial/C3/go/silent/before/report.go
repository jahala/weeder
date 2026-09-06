package report

// Total adds the rows of a report up.
func Total(rows []int) int {
	sum := 0
	for _, row := range rows {
		sum += row
	}
	return sum
}
