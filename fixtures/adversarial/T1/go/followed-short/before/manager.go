package demo

// Manager keeps one connection open.
type Manager struct{}

// Open dials the server once.
func (m *Manager) Open(dial func() bool) bool {
	_, connected := retryConnect(dial, 1)
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
