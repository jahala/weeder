package demo

// Client talks to the server.
type Client struct{}

// Connect dials the server until it answers, three tries at most.
func (c *Client) Connect(dial func() bool) bool {
	_, connected := retryConnect(dial, 3)
	return connected
}

// retryConnect dials until the server answers or the tries run out, and says
// how many tries it took and whether it connected.
func retryConnect(dial func() bool, tries int) (int, bool) {
	for attempt := 1; attempt <= tries; attempt++ {
		if dial() {
			return attempt, true
		}
	}
	return tries, false
}
