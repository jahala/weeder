package demo

// Clock answers with the time the run started.
type Clock interface {
	Now() int64
}
