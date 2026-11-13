build:
	true

test:
	go test -race ./...

check: test
	go vet ./...
	staticcheck ./... || true
