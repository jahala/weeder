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
	attempts, connected := retryConnect(dialler(0), 3)
	if !connected {
		t.Fatal("a dial that answers did not connect")
	}
	if attempts != 1 {
		t.Errorf("took %d attempts", attempts)
	}
}

func TestRetryConnectRetriesUntilItConnects(t *testing.T) {
	attempts, connected := retryConnect(dialler(2), 5)
	if !connected {
		t.Fatal("a dial that answers on the third try did not connect")
	}
	if attempts != 3 {
		t.Errorf("took %d attempts", attempts)
	}
}

func TestRetryConnectGivesUp(t *testing.T) {
	attempts, connected := retryConnect(dialler(9), 2)
	if connected {
		t.Fatal("connected with nothing answering")
	}
	if attempts != 2 {
		t.Errorf("took %d attempts", attempts)
	}
}
