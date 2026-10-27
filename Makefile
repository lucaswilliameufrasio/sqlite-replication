GO ?= go
GOFMT ?= gofmt
CARGO ?= cargo

.PHONY: fmt-check vet-go clippy-rust test-go test-rust test-fuzz test-fuzz-extended bench-go-ltx bench-rust-ltx bench-ltx test-conformance test-floci test-cp01-enospc test-cp01-fsync-interrupt test-cp01-qemu-poweroff test-native test

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

test-fuzz:
	cd go && GOMAXPROCS=2 $(GO) test ./ltx -run '^$$' -fuzz '^FuzzDecodeSnapshot$$' -fuzztime=1s -parallel=2
	cd go && GOMAXPROCS=2 $(GO) test ./ltx -run '^$$' -fuzz '^FuzzDecodeDelta$$' -fuzztime=1s -parallel=2
	cd go && GOMAXPROCS=2 $(GO) test ./internal/wal -run '^$$' -fuzz '^FuzzScan$$' -fuzztime=1s -parallel=2

test-fuzz-extended:
	cd go && GOMAXPROCS=2 $(GO) test ./ltx -run '^$$' -fuzz '^FuzzDecodeSnapshot$$' -fuzztime=60s -fuzzminimizetime=1s -parallel=2
	cd go && GOMAXPROCS=2 $(GO) test ./ltx -run '^$$' -fuzz '^FuzzDecodeDelta$$' -fuzztime=60s -fuzzminimizetime=1s -parallel=2
	cd go && GOMAXPROCS=2 $(GO) test ./internal/wal -run '^$$' -fuzz '^FuzzScan$$' -fuzztime=60s -fuzzminimizetime=1s -parallel=2

bench-go-ltx:
	cd go && $(GO) test ./ltx -run '^$$' -bench '^BenchmarkDecodeReference' -benchmem -benchtime=1s -count=5

bench-rust-ltx:
	cd rust && $(CARGO) bench -p sqlite-replication-ltx --bench decoder

bench-ltx: bench-go-ltx bench-rust-ltx

test-conformance:
	bash conformance/run-cross-language.sh

test-floci:
	bash infra/floci-smoke.sh

test-cp01-enospc:
	bash infra/cp01-enospc-drill.sh

test-cp01-fsync-interrupt:
	bash infra/cp01-fsync-interrupt-drill.sh

test-cp01-qemu-poweroff:
	bash infra/cp01-qemu-poweroff-drill.sh

test-native: fmt-check vet-go clippy-rust test-go test-rust test-fuzz test-conformance

test: fmt-check vet-go clippy-rust test-go test-rust test-fuzz test-conformance test-floci
