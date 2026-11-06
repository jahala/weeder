package demo

type width struct{ value, want string }

func widths() []width {
	return []width{{"a", "a  "}, {"abcd", "abc"}, {"abc", "abc"}}
}
