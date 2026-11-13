package demo

// Client talks to the server.
type Client struct{}

// Connect dials the server once.
func (c *Client) Connect(dial func() bool) bool {
	return dial()
}
