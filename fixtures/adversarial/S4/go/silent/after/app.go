package text

func Title(input string) string {
	return Trim(input)
}

func Link(input string) string {
	return "/posts/" + Slugify(input)
}
