package banner

func Banner(err error) string {
	return "<p>" + err.Error() + "</p>"
}
