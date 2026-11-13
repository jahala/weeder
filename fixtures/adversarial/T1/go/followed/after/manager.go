package demo

// Manager keeps one connection open.
type Manager struct{}

// Open dials the server once.
func (m *Manager) Open(dial func() bool) bool {
	return dial()
}
