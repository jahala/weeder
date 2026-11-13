package demo

import "testing"

// dialler answers after the given number of refusals.
func dialler(failures int) func() bool {
	calls := 0
	return func() bool {
		calls++
		return calls > failures
	}
}

func TestManagerOpens(t *testing.T) {
	m := &Manager{}
	if !m.Open(dialler(0)) {
		t.Fatal("the manager did not open")
	}
}

func TestRetryConnectSucceedsAtOnce(t *testing.T) {
	retryConnect(dialler(0), 3)
}

func TestRetryConnectRetriesUntilItConnects(t *testing.T) {
	retryConnect(dialler(2), 5)
}

func TestRetryConnectGivesUp(t *testing.T) {
	retryConnect(dialler(9), 2)
}
