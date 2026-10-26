GO ?= go
GOFMT ?= gofmt
CARGO ?= cargo

.PHONY: fmt-check vet-go clippy-rust test-go test-rust test-conformance test-floci test-cp01-enospc test-cp01-fsync-interrupt test-native test

fmt-check:
	cd go && test -z "$$($(GOFMT) -l $$(find . -type f -name '*.go'))"
	cd rust && $(CARGO) fmt --check

vet-go:
	cd go && $(GO) vet ./...

clippy-rust:
	cd rust && $(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

test-go:
	cd go && $(GO) test ./...

test-rust:
	cd rust && $(CARGO) test --workspace --all-features

test-conformance:
	bash conformance/run-cross-language.sh

test-floci:
	bash infra/floci-smoke.sh

test-cp01-enospc:
	bash infra/cp01-enospc-drill.sh

test-cp01-fsync-interrupt:
	bash infra/cp01-fsync-interrupt-drill.sh

test-native: fmt-check vet-go clippy-rust test-go test-rust test-conformance

test: fmt-check vet-go clippy-rust test-go test-rust test-conformance test-floci
